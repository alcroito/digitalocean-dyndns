use crate::db::types::*;
use color_eyre::eyre::Result;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::OptionalExtension;

pub fn get_domain_record(
    conn: &mut SqliteConnection,
    domain_name: &str,
    domain_record_type: &str,
) -> Result<Option<DomainRecord>> {
    use super::super::schema::domain_records::dsl::*;
    let maybe_domain_record = domain_records
        .filter(name.eq(domain_name))
        .filter(record_type.eq(domain_record_type))
        .first(conn)
        .optional()?;
    Ok(maybe_domain_record)
}

pub fn get_domain_record_by_id(
    conn: &mut SqliteConnection,
    domain_id: PrimaryKey,
) -> Result<Option<DomainRecord>> {
    use super::super::schema::domain_records::dsl::*;
    let maybe_domain_record = domain_records
        .filter(id.eq(domain_id))
        .first(conn)
        .optional()?;
    Ok(maybe_domain_record)
}

pub fn get_domain_records(conn: &mut SqliteConnection) -> Result<Vec<DomainRecord>> {
    use super::super::schema::domain_records::dsl::*;
    let results = domain_records.load::<DomainRecord>(conn)?;
    Ok(results)
}

pub fn create_domain_record(
    conn: &mut SqliteConnection,
    new_domain_record: &NewDomainRecord<'_, '_>,
) -> Result<Option<DomainRecord>> {
    use super::super::schema::domain_records;
    let domain_record =
        get_domain_record(conn, new_domain_record.name, new_domain_record.record_type);
    match domain_record {
        Ok(Some(existing)) => diesel::update(&existing)
            .set(new_domain_record)
            .execute(conn)?,
        Ok(None) => diesel::insert_into(domain_records::table)
            .values(new_domain_record)
            .execute(conn)?,
        Err(e) => return Err(e),
    };
    get_domain_record(conn, new_domain_record.name, new_domain_record.record_type)
}
