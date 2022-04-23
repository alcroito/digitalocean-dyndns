use crate::token::SecretDigitalOceanToken;
use color_eyre::eyre::Result;
use humantime::Duration;
use serde::Deserialize;
use std::time::Duration as StdDuration;

#[non_exhaustive]
#[derive(Debug)]
pub struct AppConfig {
    pub domains: Domains,
    pub general_options: GeneralOptions,
}

#[non_exhaustive]
#[derive(Debug)]
pub struct GeneralOptions {
    pub update_interval: UpdateInterval,
    // TODO: Is there a better type to use here instead of Option?
    pub digital_ocean_token: Option<SecretDigitalOceanToken>,
    pub log_level: tracing::Level,
    pub dry_run: bool,
    pub ipv4: bool,
    pub ipv6: bool,
}

#[non_exhaustive]
#[derive(Debug, Deserialize)]
pub struct DomainRecord {
    #[serde(rename(deserialize = "type"))]
    pub record_type: String,
    pub name: String,
}
#[non_exhaustive]
#[derive(Debug, Deserialize)]
pub struct Domain {
    pub name: String,
    pub records: Vec<DomainRecord>,
}

#[non_exhaustive]
#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct Domains {
    pub domains: Vec<Domain>,
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
