use anyhow::{bail, Context, Result};
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use spin_sdk::{http::conversions::IntoBody, key_value::Store};

use crate::asn_resolver;

const KEY_BLOCKED: &'static str = "blocked";

pub struct Persistence {}

impl Persistence {
    pub fn get_blocking_data() -> Result<BlockedData> {
        let store = Store::open_default()?;
        Ok(match store.get_json::<BlockedData>(KEY_BLOCKED)? {
            None => BlockedData::new(),
            Some(b) => b,
        })
    }

    pub fn add_items_to_blocklist(kind: BlockedClaimType, values: Vec<String>) -> Result<()> {
        let store = Store::open_default()?;
        let mut all = match store.get_json::<BlockedData>(KEY_BLOCKED)? {
            Some(blocking) => blocking,
            None => BlockedData::new(),
        };
        for value in values {
            if !all.contains(&kind, &value) {
                all.push(&kind, value);
            }
        }
        store
            .set_json(KEY_BLOCKED, &all)
            .with_context(|| "Error storing value in block list")
    }

    pub fn remove_items_from_blocklist(kind: BlockedClaimType, values: Vec<String>) -> Result<()> {
        let store = Store::open_default()?;
        let mut all = match store.get_json::<BlockedData>(KEY_BLOCKED)? {
            Some(blocking) => blocking,
            None => BlockedData::new(),
        };
        for value in values {
            if all.contains(&kind, &value) {
                all.retain(&kind, |v| v != &value);
            }
        }
        store
            .set_json(KEY_BLOCKED, &all)
            .with_context(|| "Error storing value in block list")
    }
}
impl Persistence {
    pub async fn add_asns_to_blocklist(values: Vec<u32>) -> Result<()> {
        let store = Store::open_default()?;
        let mut all = match store.get_json::<BlockedData>(KEY_BLOCKED)? {
            Some(blocking) => blocking,
            None => BlockedData::new(),
        };

        let futures = values
            .into_iter()
            .filter(|asn| !all.contains_asn(asn.clone()))
            .map(|asn| asn_resolver::resolve(asn));

        match try_join_all(futures).await {
            Err(e) => bail!(format!("Error while resolving CIDRs for ASN. {}", e)),
            Ok(all_asns) => all.push_asns(all_asns),
        }
        store
            .set_json(KEY_BLOCKED, &all)
            .with_context(|| "Error while updating block data in KV")
    }

    pub fn remove_asns_from_blocklist(values: Vec<u32>) -> Result<()> {
        let store = Store::open_default()?;
        let mut all = match store.get_json::<BlockedData>(KEY_BLOCKED)? {
            Some(blocking) => blocking,
            None => BlockedData::new(),
        };
        for value in values {
            if all.contains_asn(value) {
                all.retain_asn(value);
            }
        }
        store
            .set_json(KEY_BLOCKED, &all)
            .with_context(|| "Error storing value in block list")
    }
}

pub enum BlockedClaimType {
    Subject,
    Country,
    Cidr,
    UserAgent,
}

impl TryFrom<&str> for BlockedClaimType {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value.to_uppercase().as_str() {
            "SUBJECT" => Ok(BlockedClaimType::Subject),
            "COUNTRY" => Ok(BlockedClaimType::Country),
            "CIDR" => Ok(BlockedClaimType::Cidr),
            "USERAGENT" => Ok(BlockedClaimType::UserAgent),
            _ => bail!("Invalid ClaimType provided"),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct BlockedData {
    pub sub: Vec<String>,
    pub countries: Vec<String>,
    pub cidrs: Vec<String>,
    pub asns: Vec<Asn>,
    pub user_agents: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Asn {
    pub asn: u32,
    pub cidrs: Vec<String>,
}

impl BlockedData {
    fn new() -> Self {
        Self {
            asns: vec![],
            sub: vec![],
            countries: vec![],
            cidrs: vec![],
            user_agents: vec![],
        }
    }

    fn contains_asn(&self, asn: u32) -> bool {
        self.asns.iter().any(|known| known.asn == asn)
    }

    fn contains(&self, kind: &BlockedClaimType, value: &String) -> bool {
        match kind {
            BlockedClaimType::Subject => self.sub.contains(value),
            BlockedClaimType::Country => self.countries.contains(value),
            BlockedClaimType::Cidr => self.cidrs.contains(value),
            BlockedClaimType::UserAgent => self.user_agents.contains(value),
        }
    }

    fn push_asns(&mut self, mut new_asns: Vec<Asn>) {
        self.asns.append(&mut new_asns);
    }

    fn push(&mut self, kind: &BlockedClaimType, value: String) {
        match kind {
            BlockedClaimType::Subject => self.sub.push(value),
            BlockedClaimType::Country => self.countries.push(value),
            BlockedClaimType::Cidr => self.cidrs.push(value),
            BlockedClaimType::UserAgent => self.user_agents.push(value),
        }
    }

    fn retain_asn(&mut self, asn: u32) {
        self.asns.retain(|found| found.asn != asn)
    }
    fn retain<F>(&mut self, kind: &BlockedClaimType, predicate: F)
    where
        F: FnMut(&String) -> bool,
    {
        match kind {
            BlockedClaimType::Subject => self.sub.retain(predicate),
            BlockedClaimType::Country => self.countries.retain(predicate),
            BlockedClaimType::Cidr => self.cidrs.retain(predicate),
            BlockedClaimType::UserAgent => self.user_agents.retain(predicate),
        }
    }
}

impl IntoBody for BlockedData {
    fn into_body(self) -> Vec<u8> {
        serde_json::to_vec(&self)
            .with_context(|| "Error serializing BlockedData")
            .unwrap()
    }
}
