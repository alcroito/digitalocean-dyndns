use crate::db::types::*;
use color_eyre::eyre::Result;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::OptionalExtension;

pub fn get_domain_record_last_update_by_domain_record_id(
    conn: &mut SqliteConnection,
    query_domain_record_id: ForeignKey,
) -> Result<Option<DomainRecordLastUpdate>> {
    use super::super::schema::domain_record_last_updates::dsl::*;
    let result = domain_record_last_updates
        .filter(domain_record_id.eq(query_domain_record_id))
        .first(conn)
        .optional()?;
    Ok(result)
}

pub fn get_domain_record_last_update_by_id(
    conn: &mut SqliteConnection,
    query_id: PrimaryKey,
) -> Result<Option<DomainRecordLastUpdate>> {
    use super::super::schema::domain_record_last_updates::dsl::*;
    let result = domain_record_last_updates
        .filter(id.eq(query_id))
        .first(conn)
        .optional()?;
    Ok(result)
}

pub fn get_domain_record_last_updates(
    conn: &mut SqliteConnection,
) -> Result<Vec<DomainRecordLastUpdate>> {
    use super::super::schema::domain_record_last_updates::dsl::*;
    let results = domain_record_last_updates.load::<DomainRecordLastUpdate>(conn)?;
    Ok(results)
}

pub fn create_domain_record_last_update(
    conn: &mut SqliteConnection,
    new_value: &NewDomainRecordLastUpdate,
) -> Result<Option<DomainRecordLastUpdate>> {
    use super::super::schema::domain_record_last_updates;

    let domain_record_last_update =
        get_domain_record_last_update_by_domain_record_id(conn, new_value.domain_record_id);
    match domain_record_last_update {
        Ok(Some(existing)) => diesel::update(&existing).set(new_value).execute(conn)?,
        Ok(None) => diesel::insert_into(domain_record_last_updates::table)
            .values(new_value)
            .execute(conn)?,
        Err(e) => return Err(e),
    };
    get_domain_record_last_update_by_domain_record_id(conn, new_value.domain_record_id)
}
