use crate::db::types::*;
use chrono::NaiveDateTime;
use color_eyre::eyre::Result;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::OptionalExtension;

pub fn get_domain_ip_last_fetch(conn: &mut SqliteConnection) -> Result<Option<DomainIpLastFetch>> {
    use super::super::schema::domain_ip_last_fetches::dsl::*;
    let result = domain_ip_last_fetches.first(conn).optional()?;
    Ok(result)
}

pub fn get_domain_ip_last_fetch_by_id(
    conn: &mut SqliteConnection,
    query_id: PrimaryKey,
) -> Result<Option<DomainIpLastFetch>> {
    use super::super::schema::domain_ip_last_fetches::dsl::*;
    let result = domain_ip_last_fetches
        .filter(id.eq(query_id))
        .first(conn)
        .optional()?;
    Ok(result)
}

pub fn get_domain_ip_last_fetch_by_attempt_date(
    conn: &mut SqliteConnection,
    attempt_date: NaiveDateTime,
) -> Result<Option<DomainIpLastFetch>> {
    use super::super::schema::domain_ip_last_fetches::dsl::*;
    let result = domain_ip_last_fetches
        .filter(last_attempt_date.eq(attempt_date))
        .first(conn)
        .optional()?;
    Ok(result)
}

pub fn get_domain_ip_last_fetches(conn: &mut SqliteConnection) -> Result<Vec<DomainIpLastFetch>> {
    use super::super::schema::domain_ip_last_fetches::dsl::*;
    let results = domain_ip_last_fetches.load::<DomainIpLastFetch>(conn)?;
    Ok(results)
}

pub fn create_domain_ip_last_fetch(
    conn: &mut SqliteConnection,
    new_value: &NewDomainIpLastFetch,
) -> Result<Option<DomainIpLastFetch>> {
    use super::super::schema::domain_ip_last_fetches;

    let domain_ip_last_fetch = get_domain_ip_last_fetch(conn);
    match domain_ip_last_fetch {
        Ok(Some(existing)) => diesel::update(&existing).set(new_value).execute(conn)?,
        Ok(None) => diesel::insert_into(domain_ip_last_fetches::table)
            .values(new_value)
            .execute(conn)?,
        Err(e) => return Err(e),
    };
    get_domain_ip_last_fetch(conn)
}
