use color_eyre::eyre::{eyre, Result, WrapErr};
use diesel::prelude::*;
use diesel::sqlite::{Sqlite, SqliteConnection};
use diesel_migrations::{
    embed_migrations, EmbeddedMigrations, HarnessWithOutput, MigrationHarness,
};
use directories::ProjectDirs;
use tracing::trace;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

pub fn setup_db(maybe_db_path: Option<std::path::PathBuf>) -> Result<SqliteConnection> {
    let config_db_path = maybe_db_path.as_ref().map(|db_path| db_path.as_ref());
    let db_path = find_db_path(config_db_path)?;
    create_db_dir(&db_path)?;
    let mut conn = establish_connection(&db_path)?;
    run_migrations(&mut conn)?;
    Ok(conn)
}

pub fn find_db_path(config_db_path: Option<&std::path::Path>) -> Result<String> {
    if cfg!(debug_assertions) {
        let db_path = "./config/do_ddns_test.sqlite";
        trace!("Using debug database path: {}", db_path);
        return Ok(db_path.to_owned());
    }

    if let Some(db_path) = config_db_path {
        let db_path = db_path
            .to_str()
            .expect("Expecting valid UTF8 database path");
        trace!("Using database path specified via config: {}", db_path);
        Ok(db_path.to_owned())
    } else {
        // Linux:   /home/alice/.config/digitalocean-dyndns
        // Windows: C:\Users\Alice\AppData\Roaming\alcroito\digitalocean-dyndns
        // macOS:   /Users/Alice/Library/Application Support/org.alcroito.digitalocean-dyndns
        let project_dirs = ProjectDirs::from("org", "alcroito", "digitalocean-dyndns")
            .ok_or_else(|| eyre!("Could not retrieve path to store database."))?;

        let db_path: std::path::PathBuf = [
            project_dirs.data_dir(),
            std::path::Path::new("dyndns_db.sqlite"),
        ]
        .iter()
        .collect();
        let db_path = db_path
            .to_str()
            .ok_or_else(|| eyre!("Invalid characters found in auto-computed database path."))?
            .to_owned();
        trace!("Using computed database path: {}", db_path);
        Ok(db_path)
    }
}

pub fn create_db_dir(db_path: &str) -> Result<()> {
    let db_path = std::path::Path::new(db_path);
    let db_dir = db_path.parent().ok_or_else(|| {
        eyre!(
            "Invalid database path {:?}: parent is not a directory",
            db_path
        )
    })?;
    if !db_dir.exists() {
        trace!(
            "Creating directories to store the database: {}",
            db_dir
                .to_str()
                .expect("Invalid characters in database directory path")
        );
        std::fs::create_dir_all(db_dir).wrap_err("Failed to create database directory")?;
    }
    Ok(())
}

pub fn establish_connection(db_path: &str) -> Result<SqliteConnection> {
    SqliteConnection::establish(db_path).wrap_err(format!("Error connecting to {db_path}"))
}

pub fn run_migrations(conn: &mut impl MigrationHarness<Sqlite>) -> Result<()> {
    trace!("Running database migrations");
    if cfg!(debug_assertions) {
        HarnessWithOutput::write_to_stdout(conn)
            .run_pending_migrations(MIGRATIONS)
            .map_err(|e| eyre!(e))?;
    } else {
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| eyre!(e))?;
    }
    Ok(())
}
