use crate::digital_ocean_token::SecretDigitalOceanToken;
use color_eyre::eyre::Result;
use humantime::parse_duration;
use serde::{Deserialize, Serialize};
use std::{ops::Deref, sync::Arc, time::Duration};

use super::provider_config::{ProviderType, ProvidersConfig};

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
    pub general_options: GeneralOptionsApp,
}

/// The final `ConfigOptions` struct used by the app at runtime.
/// It's deserialized from the config file and command line arguments that are stored in the Figment type.
#[non_exhaustive]
#[derive(Debug, Deserialize)]
pub struct GeneralOptionsApp {
    pub update_interval: UpdateInterval,
    pub digital_ocean_token: Option<SecretDigitalOceanToken>,
    #[serde(default, flatten)]
    pub providers_config: ProvidersConfig,
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

/// The default values for `GeneralOptionsConfig`, used when no config file is provided.
#[non_exhaustive]
#[derive(Debug, Serialize)]
pub struct GeneralOptionsDefaults {
    pub update_interval: UpdateInterval,
    pub digital_ocean_token: Option<SecretDigitalOceanToken>,
    #[serde(flatten)]
    pub providers_config: ProvidersConfig,
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
            providers_config: Default::default(),
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

    /// Optional list of providers to update this record on.
    /// - `None`: update on ALL configured providers (default behavior)
    /// - `Some(vec![])`: update on ALL configured providers (explicitly saying "all")
    /// - `Some(vec![...])`: update only on specified providers
    #[serde(default)]
    pub providers: Option<Vec<ProviderType>>,
}

impl DomainRecord {
    /// Check if this record should be updated on the given provider
    ///
    /// Returns:
    /// - `true` if `providers` is `None` (default: update on all providers)
    /// - `true` if `providers` is `Some(vec![])` (explicitly saying "all providers")
    /// - `true` if `providers` contains the given provider
    /// - `false` if `providers` doesn't contain the given provider
    pub fn should_update_on(&self, provider: ProviderType) -> bool {
        match &self.providers {
            // If providers is None, update on all providers (default behavior)
            None => true,
            // If providers is Some with empty vec, also update on all providers
            Some(vec) if vec.is_empty() => true,
            // Otherwise, check if the provider is in the list
            Some(providers) => providers.contains(&provider),
        }
    }
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
    impl Visitor<'_> for LogLevelVisitor {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_record_should_update_on_none() {
        let record = DomainRecord {
            record_type: "A".to_string(),
            name: "test".to_string(),
            providers: None,
        };

        // Should update on all providers when providers is None
        assert!(record.should_update_on(ProviderType::DigitalOcean));
        assert!(record.should_update_on(ProviderType::Hetzner));
    }

    #[test]
    fn test_domain_record_should_update_on_empty() {
        let record = DomainRecord {
            record_type: "A".to_string(),
            name: "test".to_string(),
            providers: Some(vec![]),
        };

        // Should update on all providers when providers list is explicitly empty
        assert!(record.should_update_on(ProviderType::DigitalOcean));
        assert!(record.should_update_on(ProviderType::Hetzner));
    }

    #[test]
    fn test_domain_record_should_update_on_specific_provider() {
        let record = DomainRecord {
            record_type: "A".to_string(),
            name: "test".to_string(),
            providers: Some(vec![ProviderType::DigitalOcean]),
        };

        // Should only update on DigitalOcean
        assert!(record.should_update_on(ProviderType::DigitalOcean));
        assert!(!record.should_update_on(ProviderType::Hetzner));
    }

    #[test]
    fn test_domain_record_should_update_on_multiple_providers() {
        let record = DomainRecord {
            record_type: "A".to_string(),
            name: "test".to_string(),
            providers: Some(vec![ProviderType::DigitalOcean, ProviderType::Hetzner]),
        };

        // Should update on both providers
        assert!(record.should_update_on(ProviderType::DigitalOcean));
        assert!(record.should_update_on(ProviderType::Hetzner));
    }

    #[test]
    fn test_domain_record_deserialization_without_providers() {
        let toml = r#"
            type = "A"
            name = "test"
        "#;
        let record: DomainRecord = toml::from_str(toml).unwrap();
        assert_eq!(record.record_type, "A");
        assert_eq!(record.name, "test");
        assert!(record.providers.is_none());
    }

    #[test]
    fn test_domain_record_deserialization_with_providers() {
        let toml = r#"
            type = "A"
            name = "test"
            providers = ["digitalocean", "hetzner"]
        "#;
        let record: DomainRecord = toml::from_str(toml).unwrap();
        assert_eq!(record.record_type, "A");
        assert_eq!(record.name, "test");
        let providers = record.providers.unwrap();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&ProviderType::DigitalOcean));
        assert!(providers.contains(&ProviderType::Hetzner));
    }

    #[test]
    fn test_domain_record_deserialization_with_empty_providers() {
        let toml = r#"
            type = "A"
            name = "test"
            providers = []
        "#;
        let record: DomainRecord = toml::from_str(toml).unwrap();
        assert_eq!(record.record_type, "A");
        assert_eq!(record.name, "test");
        let providers = record.providers.unwrap();
        assert!(providers.is_empty());
    }

    #[test]
    fn test_domain_record_deserialization_with_single_provider() {
        let toml = r#"
            type = "A"
            name = "test"
            providers = ["hetzner"]
        "#;
        let record: DomainRecord = toml::from_str(toml).unwrap();
        assert_eq!(record.record_type, "A");
        assert_eq!(record.name, "test");
        let providers = record.providers.unwrap();
        assert_eq!(providers.len(), 1);
        assert!(providers.contains(&ProviderType::Hetzner));
    }

    #[test]
    fn test_domain_record_provider_filtering_logic() {
        // Test case 1: No providers specified - should work with all
        let record1 = DomainRecord {
            record_type: "A".to_string(),
            name: "all".to_string(),
            providers: None,
        };
        assert!(record1.should_update_on(ProviderType::DigitalOcean));
        assert!(record1.should_update_on(ProviderType::Hetzner));

        // Test case 2: Empty providers list - should work with all providers
        let record2 = DomainRecord {
            record_type: "A".to_string(),
            name: "all_explicit".to_string(),
            providers: Some(vec![]),
        };
        assert!(record2.should_update_on(ProviderType::DigitalOcean));
        assert!(record2.should_update_on(ProviderType::Hetzner));

        // Test case 3: Specific provider - should only work with that provider
        let record3 = DomainRecord {
            record_type: "A".to_string(),
            name: "do_only".to_string(),
            providers: Some(vec![ProviderType::DigitalOcean]),
        };
        assert!(record3.should_update_on(ProviderType::DigitalOcean));
        assert!(!record3.should_update_on(ProviderType::Hetzner));

        // Test case 4: Multiple providers - should work with all specified
        let record4 = DomainRecord {
            record_type: "A".to_string(),
            name: "multi".to_string(),
            providers: Some(vec![ProviderType::DigitalOcean, ProviderType::Hetzner]),
        };
        assert!(record4.should_update_on(ProviderType::DigitalOcean));
        assert!(record4.should_update_on(ProviderType::Hetzner));
    }
}
