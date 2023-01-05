use color_eyre::eyre::Result;
use std::net::IpAddr;

use crate::types::api::DomainRecords;
use crate::types::DomainRecordToUpdate;

pub trait DomainRecordApi {
    fn get_domain_records(&self, domain_name: &str) -> Result<DomainRecords>;
    fn update_domain_ip(
        &self,
        domain_record_id: u64,
        record_to_update: &DomainRecordToUpdate,
        new_ip: &IpAddr,
    ) -> Result<()>;
}

pub mod digital_ocean;
