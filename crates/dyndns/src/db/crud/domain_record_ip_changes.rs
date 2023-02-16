use crate::db::types::*;
use color_eyre::eyre::Result;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use diesel::sql_types::Text;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Queryable, Debug, Serialize, QueryableByName, JsonSchema)]
pub struct DomainRecordIpChange {
    #[diesel(embed)]
    #[serde(flatten)]
    pub domain_record_update: DomainRecordUpdate,
    #[diesel(sql_type = Text)]
    pub name: String,
}

#[derive(Serialize, JsonSchema)]
pub struct DomainRecordIpChanges {
    pub changes: Vec<DomainRecordIpChange>,
}

pub fn get_domain_record_ip_changes(conn: &mut SqliteConnection) -> Result<DomainRecordIpChanges> {
    // For each domain row, get previous ip and only return results
    // where the ip has changed.
    let changes = diesel::sql_query(
        "
        SELECT t.* FROM
        (SELECT u.*, r.*,
         lag(u.set_ip) over (
             partition by domain_record_id
             order by u.attempt_date ASC
         ) as prev_set_ip
         FROM domain_record_updates u
         INNER JOIN domain_records r
         ON u.domain_record_id=r.id
         WHERE u.success = true
         ORDER BY u.attempt_date DESC
        ) t
        WHERE prev_set_ip IS NULL OR prev_set_ip != set_ip",
    )
    .load::<DomainRecordIpChange>(conn)?;
    let results = DomainRecordIpChanges { changes };
    Ok(results)
}
