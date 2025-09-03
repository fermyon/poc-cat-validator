mod cat;
mod country;
mod header;
mod kv;
mod nip;
mod version;

use std::net::{Ipv4Addr, Ipv6Addr};

use anyhow::{anyhow, Result};
use common_access_token::CborValue;
use ipnet::IpNet;
use regex::Regex;

pub use cat::*;
pub use country::*;
pub use header::*;
pub use nip::*;
pub use version::*;

pub trait Validate {
    fn get_claim_key(&self) -> &i32;
    fn validate(&self, claim: Option<&CborValue>) -> Result<()>;
}

#[allow(dead_code)]
pub(self) trait Convert {
    fn as_str(&self) -> Option<&str>;
    fn as_string(&self) -> Option<String>;
    fn as_i64(&self) -> Option<i64>;
    fn as_match_kind(&self) -> Option<MatchKind>;
    fn as_network_address(&self) -> Option<NetworkAddress>;
    fn as_network_addresses(&self) -> Option<Vec<NetworkAddress>>;
}

pub enum NetworkAddress {
    IPv4Prefix(IpNet),
    IPv4(Ipv4Addr),
    IPv6Prefix(IpNet),
    IPv6(Ipv6Addr),
    ASN(u32),
}

pub enum MatchKind {
    Exact(String),
    Prefix(String),
    Suffix(String),
    Contains(String),
    RegEx(String),
}

impl MatchKind {
    fn validate(&self, header_value: String) -> Result<()> {
        let valid = match self {
            MatchKind::Exact(expected) => &header_value == expected,
            MatchKind::Prefix(expected) => header_value.starts_with(expected),
            MatchKind::Suffix(expected) => header_value.ends_with(expected),
            MatchKind::Contains(expected) => header_value.contains(expected),
            MatchKind::RegEx(expected) => {
                let r = Regex::new(expected).unwrap();
                r.is_match(header_value.as_str())
            }
        };
        match valid {
            true => Ok(()),
            false => Err(anyhow!("Header value not valid")),
        }
    }
}
impl Convert for CborValue {
    fn as_str(&self) -> Option<&str> {
        if let CborValue::Text(value) = self {
            return Some(value.as_str());
        }
        None
    }

    fn as_string(&self) -> Option<String> {
        if let CborValue::Text(value) = self {
            return Some(value.clone());
        }
        None
    }

    fn as_i64(&self) -> Option<i64> {
        if let CborValue::Integer(value) = self {
            return Some(value.clone());
        }
        None
    }

    fn as_match_kind(&self) -> Option<MatchKind> {
        if let CborValue::Map(value) = self {
            let Some(operant) = value.get(&1) else {
                return None;
            };
            let Some(match_value) = value.get(&2) else {
                return None;
            };

            let Some(match_value) = match_value.as_string() else {
                return None;
            };

            return match operant {
                CborValue::Integer(0) => Some(MatchKind::Exact(match_value)),
                CborValue::Integer(1) => Some(MatchKind::Prefix(match_value)),
                CborValue::Integer(2) => Some(MatchKind::Suffix(match_value)),
                CborValue::Integer(3) => Some(MatchKind::Contains(match_value)),
                CborValue::Integer(4) => Some(MatchKind::RegEx(match_value)),
                _ => None,
            };
        }
        return None;
    }

    fn as_network_addresses(&self) -> Option<Vec<NetworkAddress>> {
        if let CborValue::Array(values) = self {
            return Some(
                values
                    .iter()
                    .filter_map(|value| value.as_network_address())
                    .collect(),
            );
        }
        return None;
    }

    fn as_network_address(&self) -> Option<NetworkAddress> {
        match self {
            CborValue::Integer(_asn) => todo!("Decode asn"),
            CborValue::Bytes(ref b) if b.len() == 16 => {
                let Ok(addr): Result<[u8; 16], _> = b.as_slice().try_into() else {
                    return None;
                };
                return Some(NetworkAddress::IPv6(Ipv6Addr::from(addr)));
            }
            CborValue::Bytes(ref b) if b.len() == 4 => {
                let Ok(addr): Result<[u8; 4], _> = b.as_slice().try_into() else {
                    return None;
                };
                return Some(NetworkAddress::IPv4(Ipv4Addr::from(addr)));
            }
            CborValue::Array(ref arr) if arr.len() == 2 => {
                if let (CborValue::Integer(len), CborValue::Bytes(ref prefix_bytes)) =
                    (&arr[0], &arr[1])
                {
                    let len = *len as u8;
                    match prefix_bytes.len() {
                        0..=4 => {
                            // IPv4 prefix
                            let mut octets = [0u8; 4];
                            octets[..prefix_bytes.len()].copy_from_slice(prefix_bytes);
                            let network = Ipv4Addr::from(octets);
                            if let Ok(ipnet) = IpNet::new(network.into(), len) {
                                return Some(NetworkAddress::IPv4Prefix(ipnet));
                            };
                            return None;
                        }
                        0..=16 => {
                            // IPv6 prefix
                            let mut octets = [0u8; 16];
                            octets[..prefix_bytes.len()].copy_from_slice(prefix_bytes);
                            let network = Ipv6Addr::from(octets);
                            if let Ok(ipnet) = IpNet::new(network.into(), len) {
                                return Some(NetworkAddress::IPv6Prefix(ipnet));
                            };
                            return None;
                        }
                        _ => return None,
                    }
                };
                None
            }

            _ => {
                println!("Unknown or unsupported CBOR value");
                None
            }
        }
    }
}
