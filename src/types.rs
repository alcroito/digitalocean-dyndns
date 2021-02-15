use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct DomainRecord {
    pub id: u64,
    #[serde(rename = "type")]
    pub domain_type: String,
    pub name: String,
    pub data: String,
}

#[derive(Deserialize, Debug)]
pub struct DomainRecords {
    pub domain_records: Vec<DomainRecord>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateDomainRecordResponse {
    pub domain_record: DomainRecord,
}
