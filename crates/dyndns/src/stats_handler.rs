use color_eyre::eyre::Result;

#[cfg(feature = "stats")]
use crate::stats_handler_db::StatsHandlerDB;
use crate::{
    config::app_config::AppConfig,
    types::{IpAddrKind, IpAddrV4AndV6},
};

pub trait StatsHandler: Send {
    fn init(&mut self) -> Result<()>;
    fn handle_ip_fetch(&mut self, maybe_fetched_ips: Option<IpAddrV4AndV6>) -> Result<()>;
    fn handle_updater_attempt(
        &mut self,
        domain_record_name: &str,
        record_type: &str,
        is_domain_record_update_successful: bool,
        ip_kind: Option<IpAddrKind>,
    ) -> Result<()>;
}

pub struct StatsHandlerNop;

impl StatsHandlerNop {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for StatsHandlerNop {
    fn default() -> Self {
        Self::new()
    }
}

impl StatsHandler for StatsHandlerNop {
    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn handle_ip_fetch(&mut self, maybe_fetched_ips: Option<IpAddrV4AndV6>) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    fn handle_updater_attempt(
        &mut self,
        domain_record_name: &str,
        record_type: &str,
        is_domain_record_update_successful: bool,
        ip_kind: Option<IpAddrKind>,
    ) -> Result<()> {
        Ok(())
    }
}

pub struct StatsHandlerFactory;

impl StatsHandlerFactory {
    pub fn new_handler(config: AppConfig) -> Box<dyn StatsHandler> {
        if !config.general_options.collect_stats {
            return Box::new(StatsHandlerNop::new());
        }

        cfg_if::cfg_if! {
            if #[cfg(feature = "stats")] {
                let db_path = config.general_options.db_path.clone();
                Box::new(StatsHandlerDB::new(db_path))
            } else {
                Box::new(StatsHandlerNop::new())
            }
        }
    }
}
