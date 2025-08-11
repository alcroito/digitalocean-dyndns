use crate::db::types::*;
use color_eyre::eyre::Result;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::OptionalExtension;

pub fn get_updater_attempts_by_id(
    conn: &mut SqliteConnection,
    query_id: PrimaryKey,
) -> Result<Option<UpdaterAttempt>> {
    use super::super::schema::updater_attempts::dsl::*;
    let result = updater_attempts
        .filter(id.eq(query_id))
        .first(conn)
        .optional()?;
    Ok(result)
}

pub fn get_updater_attempts(conn: &mut SqliteConnection) -> Result<Vec<UpdaterAttempt>> {
    use super::super::schema::updater_attempts::dsl::*;
    let results = updater_attempts.load::<UpdaterAttempt>(conn)?;
    Ok(results)
}

define_sql_function! {
    /// Represents the SQL `last_insert_row()` function
    fn last_insert_rowid() -> diesel::sql_types::BigInt;
}

pub fn create_updater_attempt(
    conn: &mut SqliteConnection,
    new_value: &NewUpdaterAttempt,
) -> Result<Option<UpdaterAttempt>> {
    use super::super::schema::updater_attempts;
    diesel::insert_into(updater_attempts::table)
        .values(new_value)
        .execute(conn)?;
    let inserted_id = diesel::select(last_insert_rowid()).get_result::<PrimaryKey>(conn)?;
    get_updater_attempts_by_id(conn, inserted_id)
}
