use chrono::NaiveDateTime;
use color_eyre::eyre::{bail, Result};

use crate::db::logic::handle_ip_fetch;
use crate::db::logic::handle_updater_attempt;
use crate::db::setup::setup_db;
use crate::db::types::DomainIpFetch;
use crate::stats_handler::StatsHandler;
use crate::types::IpAddrKind;
use crate::types::IpAddrV4AndV6;
use diesel::SqliteConnection;

pub struct StatsHandlerDB {
    db_path: Option<std::path::PathBuf>,
    db_connection: Option<SqliteConnection>,
    maybe_domain_ip_fetch: Option<DomainIpFetch>,
    maybe_attempt_date: Option<NaiveDateTime>,
}

impl StatsHandlerDB {
    pub fn new(db_path: Option<std::path::PathBuf>) -> Self {
        Self {
            db_path,
            db_connection: None,
            maybe_domain_ip_fetch: None,
            maybe_attempt_date: None,
        }
    }

    pub fn new_db_connection(&self) -> Result<SqliteConnection> {
        let db_connection = setup_db(self.db_path.clone())?;
        Ok(db_connection)
    }

    fn get_db_connection(db_connection: &mut Option<SqliteConnection>) -> &mut SqliteConnection {
        db_connection
            .as_mut()
            .expect("DB Connection should exist at this point")
    }

    fn reset_state(&mut self) {
        self.maybe_domain_ip_fetch = None;
        self.maybe_attempt_date = None;
    }
}

impl StatsHandler for StatsHandlerDB {
    fn init(&mut self) -> Result<()> {
        self.db_connection = Some(self.new_db_connection()?);
        Ok(())
    }

    fn handle_ip_fetch(&mut self, maybe_fetched_ips: Option<IpAddrV4AndV6>) -> Result<()> {
        self.reset_state();

        let (maybe_domain_ip_fetch, attempt_date) = handle_ip_fetch(
            Self::get_db_connection(&mut self.db_connection),
            maybe_fetched_ips,
        )?;
        self.maybe_domain_ip_fetch = maybe_domain_ip_fetch;
        self.maybe_attempt_date = Some(attempt_date);
        if self.maybe_domain_ip_fetch.is_none() {
            bail!("Failed to create a domain ip fetch entry.");
        }
        Ok(())
    }

    fn handle_updater_attempt(
        &mut self,
        domain_record_name: &str,
        record_type: &str,
        is_domain_record_update_successful: bool,
        ip_kind: Option<IpAddrKind>,
    ) -> Result<()> {
        handle_updater_attempt(
            Self::get_db_connection(&mut self.db_connection),
            domain_record_name,
            record_type,
            self.maybe_domain_ip_fetch
                .as_ref()
                .expect("domain ip should have already been set"),
            self.maybe_attempt_date
                .expect("attempt date should have already been set"),
            is_domain_record_update_successful,
            ip_kind,
        )
    }
}
