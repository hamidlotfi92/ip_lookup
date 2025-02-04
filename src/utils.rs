use std::net::Ipv4Addr;
use std::fs::File;

use std::io::{ self, BufRead };
use crate::hashmap::IPRangeHashMap;

pub fn parse_cidr(cidr: &str) -> (u32, u8) {
    let parts: Vec<&str> = cidr.split('/').collect();
    let ip: u32 = parts[0].parse::<Ipv4Addr>().unwrap().into();
    let prefix_len: u8 = parts[1].parse().unwrap();
    (ip, prefix_len)
}

pub fn read_ip_ranges_from_file(file_path: &str, hashmap: &mut IPRangeHashMap) -> io::Result<()> {
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 4 {
            let cidr_range = parts[0].trim();
            let isp = parts[1].trim().trim_matches('"');
            let asn = parts[2].trim();
            hashmap.insert_range(cidr_range, isp, asn);
        }
    }

    Ok(())
}
