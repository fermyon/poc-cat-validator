use std::cmp::Ordering;

use anyhow::{Context, Error, Result};
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
            match all.contains(&kind, &value) {
                Ok(_) => {}
                Err(idx) => all.push(&kind, value, idx),
            }
        }
        all.optimize();
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
            match all.contains(&kind, &value) {
                Ok(idx) => all.remove_at(&kind, idx),
                Err(_) => (),
            }
        }
        all.optimize();

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
            Err(e) => {
                return Err(Error::msg(format!(
                    "Error while resolving CIDRs for ASN. {}",
                    e
                )))
            }
            Ok(all_asns) => all.push_asns(all_asns),
        }
        all.optimize();

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
        all.optimize();

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
            _ => return Err(Error::msg("Invalid ClaimType provided")),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct BlockedData {
    pub any: bool,
    pub any_asns: bool,
    pub any_cidrs: bool,
    pub any_countries: bool,
    pub any_subjects: bool,
    pub any_user_agents: bool,
    pub asns: Vec<Asn>,
    pub countries: Vec<String>,
    pub cidrs: Vec<String>,
    pub subjects: Vec<String>,
    pub user_agents: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Asn {
    pub asn: u32,
    pub cidrs: Vec<String>,
}

impl PartialOrd for Asn {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other)) // delegate to Ord
    }
}

impl Ord for Asn {
    fn cmp(&self, other: &Self) -> Ordering {
        self.asn.cmp(&other.asn) // compare by ASN only
    }
}

impl BlockedData {
    fn new() -> Self {
        Self {
            any: false,
            any_asns: false,
            any_cidrs: false,
            any_countries: false,
            any_subjects: false,
            any_user_agents: false,
            asns: vec![],
            countries: vec![],
            cidrs: vec![],
            subjects: vec![],
            user_agents: vec![],
        }
    }

    fn optimize(&mut self) {
        self.asns.sort();
        self.any_asns = !self.asns.is_empty();
        self.countries.sort();
        self.any_countries = !self.countries.is_empty();
        self.cidrs.sort();
        self.any_cidrs = !self.cidrs.is_empty();
        self.subjects.sort();
        self.any_subjects = !self.subjects.is_empty();
        self.user_agents.sort();
        self.any_user_agents = !self.user_agents.is_empty();
        self.any = self.any_asns
            || self.any_cidrs
            || self.any_countries
            || self.any_subjects
            || self.any_user_agents;
    }

    fn contains_asn(&self, asn: u32) -> bool {
        self.asns.iter().any(|known| known.asn == asn)
    }

    fn contains(&self, kind: &BlockedClaimType, value: &String) -> Result<usize, usize> {
        match kind {
            BlockedClaimType::Subject => self.subjects.binary_search(value),
            BlockedClaimType::Country => self.countries.binary_search(value),
            BlockedClaimType::Cidr => self.cidrs.binary_search(value),
            BlockedClaimType::UserAgent => self.user_agents.binary_search(value),
        }
    }

    fn push_asns(&mut self, mut new_asns: Vec<Asn>) {
        self.asns.append(&mut new_asns);
    }

    fn push(&mut self, kind: &BlockedClaimType, value: String, at_index: usize) {
        match kind {
            BlockedClaimType::Subject => self.subjects.insert(at_index, value),
            BlockedClaimType::Country => self.countries.insert(at_index, value),
            BlockedClaimType::Cidr => self.cidrs.insert(at_index, value),
            BlockedClaimType::UserAgent => self.user_agents.insert(at_index, value),
        }
    }

    fn retain_asn(&mut self, asn: u32) {
        self.asns.retain(|found| found.asn != asn)
    }
    fn remove_at(&mut self, kind: &BlockedClaimType, idx: usize) {
        _ = match kind {
            BlockedClaimType::Subject => self.subjects.remove(idx),
            BlockedClaimType::Country => self.countries.remove(idx),
            BlockedClaimType::Cidr => self.cidrs.remove(idx),
            BlockedClaimType::UserAgent => self.user_agents.remove(idx),
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
