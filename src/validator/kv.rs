use std::net::IpAddr;

use ipnet::IpNet;

use crate::persistence::BlockedData;

pub struct KvValidator {
    blocked_data: BlockedData,
}

impl From<BlockedData> for KvValidator {
    fn from(value: BlockedData) -> Self {
        Self {
            blocked_data: value,
        }
    }
}
impl KvValidator {
    pub fn is_subject_blocked(&self, value: &Option<String>, subject_required: bool) -> bool {
        if value.is_none() {
            //todo!: if value is None, should we block it
            return subject_required;
        }
        self.blocked_data.sub.contains(&value.clone().unwrap())
    }

    pub fn is_country_blocked(&self, value: &String) -> bool {
        self.blocked_data.countries.contains(value)
    }

    pub fn is_ip_blocked_by_asn(&self, value: &String) -> bool {
        let actual_ip: IpAddr = value.parse().unwrap();
        self.blocked_data
            .asns
            .clone()
            .into_iter()
            .flat_map(|defined| defined.cidrs)
            .filter_map(|v| v.parse::<IpNet>().ok())
            .any(|cidr| cidr.contains(&actual_ip))
    }
    pub fn is_ip_blocked(&self, value: &String) -> bool {
        let actual_ip: IpAddr = value.parse().unwrap();
        self.blocked_data
            .cidrs
            .iter()
            .filter_map(|v| v.parse::<IpNet>().ok())
            .any(|cidr| cidr.contains(&actual_ip))
    }

    pub fn is_user_agent_blocked(&self, value: &String) -> bool {
        self.blocked_data.user_agents.contains(value)
    }
}
