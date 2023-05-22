use crate::token::SecretDigitalOceanToken;
use color_eyre::eyre::Result;
use humantime::parse_duration;
use serde::Deserialize;
use std::{ops::Deref, sync::Arc, time::Duration};

#[derive(Debug, Clone)]
pub struct AppConfig {
    inner: Arc<AppConfigInner>,
}

impl AppConfig {
    pub fn new(inner: AppConfigInner) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

impl Deref for AppConfig {
    type Target = AppConfigInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct AppConfigInner {
    pub domains: Domains,
    pub general_options: GeneralOptions,
}

#[non_exhaustive]
#[derive(Debug)]
pub struct GeneralOptions {
    pub update_interval: UpdateInterval,
    pub digital_ocean_token: SecretDigitalOceanToken,
    pub log_level: tracing::Level,
    pub dry_run: bool,
    pub ipv4: bool,
    pub ipv6: bool,
    pub collect_stats: bool,
    pub db_path: Option<std::path::PathBuf>,
    pub enable_web: bool,
    pub listen_hostname: String,
    pub listen_port: u16,
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
        UpdateInterval(Duration::from_secs(60 * 10))
    }
}

impl std::str::FromStr for UpdateInterval {
    type Err = humantime::DurationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_duration(s).map(UpdateInterval)
    }
}
