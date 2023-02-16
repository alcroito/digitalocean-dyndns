use crate::db::schema::domain_ip_fetches;
use crate::db::schema::domain_ip_last_fetches;
use crate::db::schema::domain_record_last_updates;
use crate::db::schema::domain_record_updates;
use crate::db::schema::domain_records;
use crate::db::schema::updater_attempts;

use chrono::NaiveDateTime;
#[cfg(feature = "web")]
use schemars::JsonSchema;
use serde::Serialize;

pub type PrimaryKey = i64;
pub type ForeignKey = i64;
pub type DBIntegerType = i64;
pub type AffectedRowCount = usize;

#[derive(Identifiable, Queryable, Debug, Serialize)]
pub struct DomainRecord {
    pub id: PrimaryKey,
    pub name: String,
    pub record_type: String,
}

#[derive(Insertable, Debug, AsChangeset)]
#[diesel(table_name = domain_records)]
pub struct NewDomainRecord<'a, 'b> {
    pub name: &'a str,
    pub record_type: &'b str,
}

#[derive(Identifiable, Queryable, Debug)]
#[diesel(table_name = domain_ip_fetches)]
pub struct DomainIpFetch {
    pub id: PrimaryKey,
    pub attempt_date: NaiveDateTime,
    pub success: bool,
    pub fetched_ipv4: Option<String>,
    pub fetched_ipv6: Option<String>,
}

#[derive(Insertable, Debug, AsChangeset)]
#[diesel(table_name = domain_ip_fetches)]
pub struct NewDomainIpFetch {
    pub attempt_date: NaiveDateTime,
    pub success: bool,
    pub fetched_ipv4: Option<String>,
    pub fetched_ipv6: Option<String>,
}

#[derive(Identifiable, Queryable, Debug)]
#[diesel(table_name = domain_ip_last_fetches)]
pub struct DomainIpLastFetch {
    pub id: PrimaryKey,
    pub attempt_count: DBIntegerType,
    pub success_count: DBIntegerType,
    pub fail_count: DBIntegerType,
    pub last_attempt_date: NaiveDateTime,
    pub last_success_date: Option<NaiveDateTime>,
    pub last_successful_fetched_ipv4: Option<String>,
    pub last_successful_fetched_ipv4_change_date: NaiveDateTime,
    pub last_successful_fetched_ipv6: Option<String>,
    pub last_successful_fetched_ipv6_change_date: NaiveDateTime,
}

#[derive(Insertable, Debug, AsChangeset)]
#[diesel(table_name = domain_ip_last_fetches)]
pub struct NewDomainIpLastFetch {
    pub attempt_count: DBIntegerType,
    pub success_count: DBIntegerType,
    pub fail_count: DBIntegerType,
    pub last_attempt_date: NaiveDateTime,
    pub last_success_date: Option<NaiveDateTime>,
    pub last_successful_fetched_ipv4: Option<String>,
    pub last_successful_fetched_ipv4_change_date: NaiveDateTime,
    pub last_successful_fetched_ipv6: Option<String>,
    pub last_successful_fetched_ipv6_change_date: NaiveDateTime,
}

#[cfg_attr(feature = "web", derive(JsonSchema))]
#[derive(Identifiable, Queryable, QueryableByName, Debug, Serialize)]
#[diesel(table_name = domain_record_updates)]
pub struct DomainRecordUpdate {
    pub id: PrimaryKey,
    pub domain_record_id: ForeignKey,
    pub set_ip: String,
    pub attempt_date: NaiveDateTime,
    pub success: bool,
}

#[derive(Insertable, Debug, AsChangeset)]
#[diesel(table_name = domain_record_updates)]
pub struct NewDomainRecordUpdate {
    pub domain_record_id: ForeignKey,
    pub set_ip: String,
    pub attempt_date: NaiveDateTime,
    pub success: bool,
}

#[derive(Identifiable, Queryable, Debug)]
#[diesel(table_name = domain_record_last_updates)]
pub struct DomainRecordLastUpdate {
    pub id: PrimaryKey,
    pub domain_record_id: ForeignKey,
    pub attempt_count: DBIntegerType,
    pub success_count: DBIntegerType,
    pub fail_count: DBIntegerType,
    pub last_attempt_date: NaiveDateTime,
    pub last_success_date: Option<NaiveDateTime>,
    pub last_set_ip: String,
}

#[derive(Insertable, Debug, AsChangeset)]
#[diesel(table_name = domain_record_last_updates)]
pub struct NewDomainRecordLastUpdate {
    pub domain_record_id: ForeignKey,
    pub attempt_count: DBIntegerType,
    pub success_count: DBIntegerType,
    pub fail_count: DBIntegerType,
    pub last_attempt_date: NaiveDateTime,
    pub last_success_date: Option<NaiveDateTime>,
    pub last_set_ip: String,
}

#[derive(Identifiable, Queryable, Debug, Serialize)]
pub struct UpdaterAttempt {
    pub id: PrimaryKey,
    pub domain_record_id: ForeignKey,
    pub domain_ip_fetches_id: ForeignKey,
    pub domain_record_updates_id: Option<ForeignKey>,
    pub attempt_date: NaiveDateTime,
}

#[derive(Insertable, Debug, AsChangeset)]
#[diesel(table_name = updater_attempts)]
pub struct NewUpdaterAttempt {
    pub domain_record_id: ForeignKey,
    pub domain_ip_fetches_id: ForeignKey,
    pub domain_record_updates_id: Option<ForeignKey>,
    pub attempt_date: NaiveDateTime,
}
