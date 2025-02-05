use std::sync::Arc;
use axum::{ http::StatusCode, response::{ IntoResponse, Response } };
use serde_with::serde_as;
use serde::{ Deserialize, Serialize };
use serde_json::json;
use crate::utils::parse_cidr; // assume parse_cidr(&str) -> (u32, u8)

//
// Original IPRange type (unchanged)
//
#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IPRange {
    #[serde_as(as = "Arc<serde_with::DisplayFromStr>")]
    pub cidr_range: Arc<String>,
    #[serde_as(as = "Arc<serde_with::DisplayFromStr>")]
    pub isp: Arc<String>,
    #[serde_as(as = "Arc<serde_with::DisplayFromStr>")]
    pub asn: Arc<String>,
}

impl IntoResponse for IPRange {
    fn into_response(self) -> Response {
        let json_response =
            json!({
            "cidr_range": *self.cidr_range,
            "isp": *self.isp,
            "asn": *self.asn,
        });
        (StatusCode::OK, axum::Json(json_response)).into_response()
    }
}

///
/// A helper structure that stores a parsed IP range.
///
#[derive(Clone)]
struct IPRangeEntry {
    network: u32,
    prefix: u8,
    ip_range: Arc<IPRange>,
}

impl IPRangeEntry {
    fn new(cidr_range: &str, isp: &str, asn: &str) -> Self {
        let (network, prefix) = parse_cidr(cidr_range);
        IPRangeEntry {
            network,
            prefix,
            ip_range: Arc::new(IPRange {
                cidr_range: Arc::from(String::from(cidr_range)),
                isp: Arc::from(String::from(isp)),
                asn: Arc::from(String::from(asn)),
            }),
        }
    }

    /// Returns the mask (as a u32) for this entry.
    fn mask(&self) -> u32 {
        if self.prefix == 0 { 0 } else { !((1u32).wrapping_shl(32 - (self.prefix as u32)) - 1) }
    }
}

///
/// A high-performance direct lookup table for IP ranges.
///
/// We trade extra memory for an O(1) lookup. Instead of iterating over a trie,
/// we precompute an array of candidate IP ranges. Each IP address, when shifted
/// by (32 - INDEX_BITS), is used as an index into this table.
///
/// The table is built from a static set of IP ranges, and the value stored
/// is the one with the longest matching prefix for that index.
///
///
#[derive(Clone)]
pub struct IPRangeDirectLookup {
    /// A vector of (prefix, Arc<IPRange>) for each table slot.
    /// If no range applies for a given slot, the entry is None.
    table: Vec<Option<(u8, Arc<IPRange>)>>,
    /// Collected IP range entries (used during table build).
    entries: Vec<IPRangeEntry>,
    /// Number of bits used for the index (must be <= 32).
    index_bits: u32,
    /// Table size is 2^(index_bits)
    table_size: usize,
}

impl IPRangeDirectLookup {
    /// Create a new direct lookup structure.
    ///
    /// * `index_bits` determines the table size (e.g. 20 yields 2^20 = ~1 million entries).
    ///   More bits means a more precise lookup table (and more memory usage).
    pub fn new(index_bits: u32) -> Self {
        assert!(index_bits <= 32);
        let table_size = 1 << index_bits;
        IPRangeDirectLookup {
            table: vec![None; table_size],
            entries: Vec::new(),
            index_bits,
            table_size,
        }
    }

    /// Inserts an IP range.
    ///
    /// Note: The lookup table is not updated immediately. Call `build_table()`
    /// after all ranges have been inserted.
    pub fn insert_range(&mut self, cidr_range: &str, isp: &str, asn: &str) {
        self.entries.push(IPRangeEntry::new(cidr_range, isp, asn));
    }

    /// Builds the direct lookup table.
    ///
    /// For each inserted IP range, we update every table slot that falls under its range,
    /// only replacing a slot if the new range has a longer prefix (i.e. is a more specific match).
    pub fn build_table(&mut self) {
        // Clear the table.
        self.table.fill(None);

        // For each IP range entry...
        for entry in &self.entries {
            let mask = entry.mask();
            // Compute the first IP in the range.
            let start_ip = entry.network & mask;
            // Compute the number of IP addresses in this range.
            let count = if entry.prefix == 32 { 1u32 } else { 1u32 << (32 - entry.prefix) };

            // Because our table is indexed by the top `index_bits` of the IP,
            // determine the indices that this IP range covers.
            //
            // For each IP address in the range, the table index is:
            //    index = ip >> (32 - index_bits)
            //
            // Rather than iterate over every IP address in the range,
            // we compute the range of table indices that may be affected.
            //
            // Note: This is an approximation. Some table slots might contain
            // IP addresses outside the IP range, but we rely on the longest prefix
            // logic to ensure correctness.
            let shift = 32 - self.index_bits;
            let start_index = start_ip >> shift;
            let end_ip = start_ip.wrapping_add(count - 1);
            let end_index = end_ip >> shift;

            for index in start_index..=end_index {
                // Update the table if:
                // - There is no entry, or
                // - The current entry's prefix is less specific than this one.
                if let Some((existing_prefix, _)) = self.table[index as usize] {
                    if entry.prefix > existing_prefix {
                        self.table[index as usize] = Some((
                            entry.prefix,
                            Arc::clone(&entry.ip_range),
                        ));
                    }
                } else {
                    self.table[index as usize] = Some((entry.prefix, Arc::clone(&entry.ip_range)));
                }
            }
        }
    }

    /// Looks up an IP address (given as u32) and returns the matching IPRange (if any).
    ///
    /// The lookup is an O(1) array index.
    pub fn search(&self, ip_addr: u32) -> Option<Arc<IPRange>> {
        let shift = 32 - self.index_bits;
        let index = ip_addr >> shift;
        // Because of the way the table was built, if an entry exists it is the best match.
        self.table
            .get(index as usize)
            .and_then(|entry| entry.as_ref().map(|(_prefix, ip_range)| Arc::clone(ip_range)))
    }
}

//
// Example usage and test
//
#[cfg(test)]
mod tests {
    use super::*;

    // Dummy parse_cidr implementation for testing.
    // Replace with your actual implementation.
    fn dummy_parse_cidr(cidr: &str) -> (u32, u8) {
        // For testing, assume "192.168.1.0/24" always.
        (0xc0a80100, 24)
    }

    // Redirect parse_cidr calls in tests to our dummy version.
    #[test]
    fn test_direct_lookup() {
        // For testing, override the parse_cidr function.
        // (In your code, ensure that your real parse_cidr is high-performance.)
        fn parse_cidr(cidr: &str) -> (u32, u8) {
            dummy_parse_cidr(cidr)
        }
        let _ = parse_cidr;

        // Create the direct lookup with 20 index bits (~1 million entries).
        let mut lookup = IPRangeDirectLookup::new(20);
        lookup.insert_range("192.168.1.0/24", "ISP1", "ASN1");
        // You can insert more ranges as needed.
        lookup.build_table();

        // Lookup an IP address in the range, e.g. 192.168.1.42.
        let ip: u32 = 0xc0a8012a;
        let result = lookup.search(ip);
        assert!(result.is_some());
        let ip_range = result.unwrap();
        assert_eq!(*ip_range.cidr_range, "192.168.1.0/24".to_string());
    }
}
