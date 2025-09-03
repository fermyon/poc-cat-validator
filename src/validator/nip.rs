use std::net::IpAddr;

use anyhow::bail;
use common_access_token::cat_keys;

use crate::validator::{Convert, Validate};

pub struct CatNipValidator {
    pub client_ip: String,
}

impl Validate for CatNipValidator {
    fn get_claim_key(&self) -> &i32 {
        &cat_keys::CATNIP
    }

    fn validate(&self, claim: Option<&common_access_token::CborValue>) -> anyhow::Result<()> {
        let Ok(ip): Result<IpAddr, _> = self.client_ip.parse() else {
            bail!("Invalid IP address received");
        };
        if claim.is_none() {
            return Ok(());
        }

        let claim = claim.unwrap();
        match claim.as_network_addresses() {
            None => Ok(()),
            Some(valid_ranges) => {
                let valid = valid_ranges.iter().any(|range| match range {
                    super::NetworkAddress::IPv4Prefix(ip_net) => ip_net.contains(&ip),
                    super::NetworkAddress::IPv4(ipv4_addr) => ipv4_addr.eq(&ip),
                    super::NetworkAddress::IPv6Prefix(ip_net) => ip_net.contains(&ip),
                    super::NetworkAddress::IPv6(ipv6_addr) => ipv6_addr.eq(&ip),
                    super::NetworkAddress::ASN(_) => todo!(),
                });
                if valid {
                    return Ok(());
                }
                bail!("IP address not allowed by token")
            }
        }
    }
}
