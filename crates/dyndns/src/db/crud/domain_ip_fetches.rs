use crate::db::types::*;
use color_eyre::eyre::Result;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::OptionalExtension;

pub fn get_domain_ip_fetch_by_id(
    conn: &mut SqliteConnection,
    query_id: PrimaryKey,
) -> Result<Option<DomainIpFetch>> {
    use super::super::schema::domain_ip_fetches::dsl::*;
    let result = domain_ip_fetches
        .filter(id.eq(query_id))
        .first(conn)
        .optional()?;
    Ok(result)
}

pub fn get_domain_ip_fetches(conn: &mut SqliteConnection) -> Result<Vec<DomainIpFetch>> {
    use super::super::schema::domain_ip_fetches::dsl::*;
    let results = domain_ip_fetches.load::<DomainIpFetch>(conn)?;
    Ok(results)
}

define_sql_function! {
    /// Represents the SQL last_insert_row() function
    fn last_insert_rowid() -> diesel::sql_types::BigInt;
}

pub fn create_domain_ip_fetch(
    conn: &mut SqliteConnection,
    new_value: &NewDomainIpFetch,
) -> Result<Option<DomainIpFetch>> {
    use super::super::schema::domain_ip_fetches;
    diesel::insert_into(domain_ip_fetches::table)
        .values(new_value)
        .execute(conn)?;
    let inserted_id = diesel::select(last_insert_rowid()).get_result::<PrimaryKey>(conn)?;
    get_domain_ip_fetch_by_id(conn, inserted_id)
}
