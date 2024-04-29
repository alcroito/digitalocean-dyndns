use crate::token::SecretDigitalOceanToken;
use color_eyre::eyre::Result;
use humantime::parse_duration;
use serde::{Deserialize, Serialize};
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
#[derive(Debug, Deserialize)]
pub struct GeneralOptions {
    pub update_interval: UpdateInterval,
    pub digital_ocean_token: SecretDigitalOceanToken,
    #[serde(deserialize_with = "deserialize_log_level_from_u8_or_string")]
    pub log_level: tracing::Level,
    pub dry_run: bool,
    pub ipv4: bool,
    pub ipv6: bool,
    pub collect_stats: bool,
    #[serde(rename = "database_path")]
    pub db_path: Option<std::path::PathBuf>,
    pub enable_web: bool,
    pub listen_hostname: String,
    pub listen_port: u16,
}

#[non_exhaustive]
#[derive(Debug, Serialize)]
pub struct GeneralOptionsDefaults {
    pub update_interval: UpdateInterval,
    pub digital_ocean_token: Option<SecretDigitalOceanToken>,
    #[serde(serialize_with = "serialize_to_u8_from_log_level")]
    pub log_level: tracing::Level,
    pub dry_run: bool,
    pub ipv4: bool,
    pub ipv6: bool,
    pub collect_stats: bool,
    #[serde(rename = "database_path")]
    pub db_path: Option<std::path::PathBuf>,
    pub enable_web: bool,
    pub listen_hostname: String,
    pub listen_port: u16,
}

impl Default for GeneralOptionsDefaults {
    fn default() -> Self {
        Self {
            update_interval: Default::default(),
            digital_ocean_token: None,
            log_level: tracing::Level::INFO,
            dry_run: Default::default(),
            ipv4: true,
            ipv6: Default::default(),
            collect_stats: Default::default(),
            db_path: Default::default(),
            enable_web: Default::default(),
            listen_hostname: "localhost".to_owned(),
            listen_port: 8095,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SimpleModeDomainConfig {
    pub domain_root: String,
    pub subdomain_to_update: Option<String>,
    pub update_domain_root: Option<bool>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateInterval(#[serde(with = "humantime_serde")] pub Duration);

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

impl std::fmt::Display for UpdateInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", humantime::format_duration(self.0))
    }
}

pub fn deserialize_log_level_from_u8_or_string<'de, D>(
    deserializer: D,
) -> Result<tracing::Level, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct LogLevelVisitor;
    impl<'de> Visitor<'de> for LogLevelVisitor {
        type Value = tracing::Level;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a number between 0 and 3 or one of the following strings: error, warn, info, debug, trace")
        }

        fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let level = match value {
                0 => tracing::Level::INFO,
                1 => tracing::Level::DEBUG,
                _ => tracing::Level::TRACE,
            };
            Ok(level)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<tracing::Level>().map_err(|e| {
                let msg = format!(
                    "error parsing log level: expected one of \"error\", \"warn\", \
                \"info\", \"debug\", \"trace\": {e}"
                );
                E::custom(msg)
            })
        }
    }

    // deserialize_u8 is just a hint, the deserializer can handle strings too.
    deserializer.deserialize_u8(LogLevelVisitor)
}

pub fn serialize_to_u8_from_log_level<S>(
    level: &tracing::Level,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let level_u8 = match *level {
        tracing::Level::INFO | tracing::Level::WARN | tracing::Level::ERROR => 0,
        tracing::Level::DEBUG => 1,
        tracing::Level::TRACE => 2,
    };
    serializer.serialize_u8(level_u8)
}
