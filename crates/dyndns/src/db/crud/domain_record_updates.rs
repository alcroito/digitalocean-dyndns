use crate::db::types::*;
use color_eyre::eyre::Result;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::OptionalExtension;

pub fn get_domain_record_update_by_id(
    conn: &mut SqliteConnection,
    query_id: PrimaryKey,
) -> Result<Option<DomainRecordUpdate>> {
    use super::super::schema::domain_record_updates::dsl::*;
    let result = domain_record_updates
        .filter(id.eq(query_id))
        .first(conn)
        .optional()?;
    Ok(result)
}

pub fn get_domain_record_updates(conn: &mut SqliteConnection) -> Result<Vec<DomainRecordUpdate>> {
    use super::super::schema::domain_record_updates::dsl::*;
    let results = domain_record_updates.load::<DomainRecordUpdate>(conn)?;
    Ok(results)
}

define_sql_function! {
    /// Represents the SQL `last_insert_row()` function
    fn last_insert_rowid() -> diesel::sql_types::BigInt;
}

pub fn create_domain_record_update(
    conn: &mut SqliteConnection,
    new_value: &NewDomainRecordUpdate,
) -> Result<Option<DomainRecordUpdate>> {
    use super::super::schema::domain_record_updates;
    diesel::insert_into(domain_record_updates::table)
        .values(new_value)
        .execute(conn)?;
    let inserted_id = diesel::select(last_insert_rowid()).get_result::<PrimaryKey>(conn)?;
    get_domain_record_update_by_id(conn, inserted_id)
}
