use anyhow::Result;
use humantime::Duration;
use std::time::Duration as StdDuration;

#[non_exhaustive]
#[derive(Debug)]
pub struct Config {
    pub domain_root: String,
    pub subdomain_to_update: String,
    pub update_interval: UpdateInterval,
    pub digital_ocean_token: String,
    pub log_level: log::LevelFilter,
    pub dry_run: bool,
}

#[derive(Clone, Debug)]
pub struct UpdateInterval(pub Duration);

impl Default for UpdateInterval {
    fn default() -> Self {
        UpdateInterval(StdDuration::from_secs(60 * 10).into())
    }
}

impl std::str::FromStr for UpdateInterval {
    type Err = humantime::DurationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Duration>().map(UpdateInterval)
    }
}
