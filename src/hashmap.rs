use std::{ collections::HashMap, sync::Arc };
use axum::{ http::StatusCode, response::{ IntoResponse, Response } };
use serde_with::serde_as;
use serde::{ Deserialize, Serialize };
use serde_json::json;
use crate::utils::parse_cidr;

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

#[derive(Clone)]
pub struct IPRangeHashMap {
    ranges: HashMap<u32, Vec<(u32, Arc<IPRange>)>>,
}

impl IPRangeHashMap {
    pub fn new() -> Self {
        IPRangeHashMap { ranges: HashMap::new() }
    }

    pub fn insert_range(&mut self, cidr_range: &str, isp: &str, asn: &str) {
        //spreate the ip address and mask and convert them into u32 and u8 types
        // for example 192.168.1.0 becomes 0xC0A80100 and /24 becomes 24
        let (network, prefix_len) = parse_cidr(cidr_range);

        // mask is a bitmask that represents cidr
        let mask = if prefix_len == 0 {
            0
        } else {
            // here is a bit complicated
            // 1 in u32 type shifts left by amount of prefix_len bits then subtracted by 1 then used in bitwise NOT operation, to created a mask used to exreact network address later, since we save the whole range as u32 for hashmap key we need it when we look up for the address in later
            !((1u32).wrapping_shl(32 - (prefix_len as u32)) - 1)
        };
        // creating unique key from bitwise & from the ip adrress and mask we just made
        let key = network & mask;
        let range = Arc::new(IPRange {
            cidr_range: Arc::from(String::from(cidr_range)),
            isp: Arc::from(String::from(isp)),
            asn: Arc::from(String::from(asn)),
        });
        // adding the range with created key to the hashmap if not exist
        self.ranges.entry(key).or_insert_with(Vec::new).push((mask, range));
    }
    pub fn search(&self, ip_addr: u32) -> Option<Arc<IPRange>> {
        // iterates over bits with bitwise operation from 32 to 0 until it finds the smalest range
        for i in (0..=32).rev() {
            // find the ip range with the given cird using the the bitwise NOT operation and use bitwise AND to see if the range for given ip address exists
            let key = ip_addr & !((1u32).wrapping_shl(i) - 1);
            if let Some(ranges) = self.ranges.get(&key) {
                for &(mask, ref range) in ranges {
                    if (ip_addr & mask) == key {
                        return Some(Arc::clone(range));
                    }
                }
            }
        }
        None
    }
}
