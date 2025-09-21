use color_eyre::eyre::Result;
use humantime::parse_duration;
use serde::{Deserialize, Serialize};
use std::{ops::Deref, sync::Arc, time::Duration};

use super::provider_config::{ProviderType, ProvidersConfig, SecretProviderToken};

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
    pub digital_ocean_token: Option<SecretProviderToken>,
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
    pub update_all_providers_by_default: bool,
}

#[non_exhaustive]
#[derive(Debug, Serialize)]
pub struct GeneralOptionsDefaults {
    pub update_interval: UpdateInterval,
    pub digital_ocean_token: Option<SecretProviderToken>,
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
    pub update_all_providers_by_default: bool,
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
            update_all_providers_by_default: true,
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
    #[serde(default)]
    pub providers: Option<Vec<ProviderType>>,
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

    use super::super::provider_config::ProviderConfig;
    use figment::providers::{Format, Toml};
    use figment::Figment;
    use secrecy::ExposeSecret;

    #[test]
    fn test_provider_config_parsing_single_provider() {
        let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "test_token_123"
    "#;

        let figment = Figment::from(Toml::string(toml));
        let config: ProvidersConfig = figment.extract().unwrap();

        assert_eq!(config.providers.len(), 1);
        assert_eq!(
            config.providers.first().unwrap().provider,
            ProviderType::DigitalOcean
        );
        assert_eq!(
            config
                .providers
                .first()
                .unwrap()
                .token
                .expose_secret()
                .as_str(),
            "test_token_123"
        );
    }

    #[test]
    fn test_provider_config_parsing_multiple_providers() {
        let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "do_token_abc123"

        [[providers]]
        provider = "hetzner"
        token = "hz_token_xyz789"
    "#;

        let figment = Figment::from(Toml::string(toml));
        let config: ProvidersConfig = figment.extract().unwrap();

        assert_eq!(config.providers.len(), 2);

        assert_eq!(
            config.providers.first().unwrap().provider,
            ProviderType::DigitalOcean
        );
        assert_eq!(
            config
                .providers
                .first()
                .unwrap()
                .token
                .expose_secret()
                .as_str(),
            "do_token_abc123"
        );

        assert_eq!(
            config.providers.get(1).unwrap().provider,
            ProviderType::Hetzner
        );
        assert_eq!(
            config
                .providers
                .get(1)
                .unwrap()
                .token
                .expose_secret()
                .as_str(),
            "hz_token_xyz789"
        );
    }

    #[test]
    fn test_provider_config_digitalocean_alias() {
        let toml = r#"
        [[providers]]
        provider = "digital_ocean"
        token = "test_token"
    "#;

        let figment = Figment::from(Toml::string(toml));
        let config: ProvidersConfig = figment.extract().unwrap();

        assert_eq!(config.providers.len(), 1);
        assert_eq!(
            config.providers.first().unwrap().provider,
            ProviderType::DigitalOcean
        );
    }

    #[test]
    fn test_empty_providers_config() {
        let toml = r#"
        # Empty config
    "#;

        let figment = Figment::from(Toml::string(toml));
        let config: ProvidersConfig = figment.extract().unwrap();

        assert_eq!(config.providers.len(), 0);
    }

    #[test]
    fn test_domain_record_no_providers_field() {
        let toml = r#"
        type = "A"
        name = "home"
    "#;

        let record: DomainRecord = toml::from_str(toml).unwrap();

        assert_eq!(record.record_type, "A");
        assert_eq!(record.name, "home");
    }

    #[test]
    fn test_domain_record_empty_providers_array() {
        let toml = r#"
        type = "A"
        name = "home"
        providers = []
    "#;

        let record: DomainRecord = toml::from_str(toml).unwrap();
        assert_eq!(record.record_type, "A");
        assert_eq!(record.name, "home");
        assert_eq!(record.providers.unwrap().len(), 0);
    }

    #[test]
    fn test_domain_record_single_provider() {
        let toml = r#"
        type = "A"
        name = "backup"
        providers = ["hetzner"]
    "#;

        let record: DomainRecord = toml::from_str(toml).unwrap();
        assert_eq!(record.record_type, "A");
        assert_eq!(record.name, "backup");
        assert_eq!(record.providers.unwrap(), vec![ProviderType::Hetzner]);
    }

    #[test]
    fn test_domain_record_multiple_providers() {
        let toml = r#"
        type = "A"
        name = "cdn"
        providers = ["digitalocean", "hetzner"]
    "#;

        let record: DomainRecord = toml::from_str(toml).unwrap();
        assert_eq!(record.record_type, "A");
        assert_eq!(record.name, "cdn");
        assert_eq!(
            record.providers.unwrap(),
            vec![ProviderType::DigitalOcean, ProviderType::Hetzner]
        );
    }

    #[test]
    fn test_domain_record_digitalocean_only() {
        let toml = r#"
        type = "A"
        name = "test"
        providers = ["digitalocean"]
    "#;

        let record: DomainRecord = toml::from_str(toml).unwrap();
        assert_eq!(record.record_type, "A");
        assert_eq!(record.name, "test");
        assert_eq!(record.providers.unwrap(), vec![ProviderType::DigitalOcean]);
    }

    #[test]
    fn test_domain_record_invalid_provider_name() {
        let toml = r#"
        type = "A"
        name = "test"
        providers = ["invalid_provider"]
    "#;

        let result: Result<DomainRecord, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_domain_record_mixed_valid_and_invalid_providers() {
        let toml = r#"
        type = "A"
        name = "test"
        providers = ["digitalocean", "cloudflare"]
    "#;

        // Should fail parsing because cloudflare is not a valid provider (yet)
        let result: Result<DomainRecord, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_providers_config_validate_empty_fails() {
        let config = ProvidersConfig::default();
        let result = config.validate();

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("At least one DNS provider must be configured"));
    }

    #[test]
    fn test_providers_config_validate_with_provider_succeeds() {
        let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "test_token"
    "#;

        let config: ProvidersConfig = toml::from_str(toml).unwrap();
        let result = config.validate();

        assert!(result.is_ok());
    }

    #[test]
    fn test_provider_config_missing_token_field() {
        let toml = r#"
        [[providers]]
        provider = "digitalocean"
    "#;

        let result: Result<ProvidersConfig, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_config_missing_provider_field() {
        let toml = r#"
        [[providers]]
        token = "test_token"
    "#;

        let result: Result<ProvidersConfig, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_config_invalid_provider_type() {
        let toml = r#"
        [[providers]]
        provider = "cloudflare"
        token = "test_token"
    "#;

        // Should fail because cloudflare is not a valid ProviderType (yet)
        let result: Result<ProvidersConfig, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    // Complex Configuration Tests

    #[test]
    fn test_full_config_with_domains_and_providers() {
        let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "do_token_123"

        [[providers]]
        provider = "hetzner"
        token = "hz_token_456"

        [[domains]]
        name = "example.com"

        [[domains.records]]
        type = "A"
        name = "home"

        [[domains.records]]
        type = "A"
        name = "backup"
        providers = ["hetzner"]

        [[domains.records]]
        type = "A"
        name = "cdn"
        providers = ["digitalocean", "hetzner"]
    "#;

        // Parse providers config
        #[derive(serde::Deserialize)]
        struct TestConfigWithProviders {
            #[serde(default)]
            providers: Vec<ProviderConfig>,
        }

        let figment = Figment::from(Toml::string(toml));
        let config_with_providers: TestConfigWithProviders = figment.extract().unwrap();
        let providers_config = ProvidersConfig {
            providers: config_with_providers.providers,
        };

        assert_eq!(providers_config.providers.len(), 2);
        assert!(providers_config.has_provider(ProviderType::DigitalOcean));
        assert!(providers_config.has_provider(ProviderType::Hetzner));

        // Parse domain records
        #[derive(serde::Deserialize)]
        struct TestDomain {
            #[allow(dead_code)]
            name: String,
            records: Vec<DomainRecord>,
        }

        #[derive(serde::Deserialize)]
        struct TestConfig {
            domains: Vec<TestDomain>,
        }

        let config: TestConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.domains.len(), 1);
        assert_eq!(config.domains.first().unwrap().records.len(), 3);

        let home_record = &config.domains.first().unwrap().records.first().unwrap();
        assert_eq!(home_record.name, "home");

        let backup_record = &config.domains.first().unwrap().records.get(1).unwrap();
        assert_eq!(backup_record.name, "backup");

        let cdn_record = &config.domains.first().unwrap().records.get(2).unwrap();
        assert_eq!(cdn_record.name, "cdn");
    }

    #[test]
    fn test_multiple_domains_with_provider_filtering() {
        let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "do_token"

        [[providers]]
        provider = "hetzner"
        token = "hz_token"

        [[domains]]
        name = "example.com"

        [[domains.records]]
        type = "A"
        name = "www"
        providers = ["digitalocean"]

        [[domains]]
        name = "example.org"

        [[domains.records]]
        type = "A"
        name = "www2"
        providers = ["hetzner"]
    "#;

        #[derive(serde::Deserialize)]
        struct TestDomain {
            name: String,
            records: Vec<DomainRecord>,
        }

        #[derive(serde::Deserialize)]
        struct TestConfig {
            providers: Vec<ProviderConfig>,
            domains: Vec<TestDomain>,
        }

        let config: TestConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.providers.len(), 2);
        assert_eq!(config.domains.len(), 2);

        assert_eq!(config.domains.first().unwrap().name, "example.com");
        assert_eq!(
            config
                .domains
                .first()
                .unwrap()
                .records
                .first()
                .unwrap()
                .name,
            "www"
        );

        assert_eq!(config.domains.get(1).unwrap().name, "example.org");
        assert_eq!(
            config.domains.get(1).unwrap().records.first().unwrap().name,
            "www2"
        );
    }

    #[test]
    fn test_duplicate_provider_types_validation_fails() {
        // Test that duplicate provider types are rejected during validation
        let toml = r#"
        [[providers]]
        provider = "digitalocean"
        token = "token1"

        [[providers]]
        provider = "digitalocean"
        token = "token2"
    "#;

        let config: ProvidersConfig = toml::from_str(toml).unwrap();

        // Should parse successfully (2 providers)
        assert_eq!(config.providers.len(), 2);

        // But validation should fail due to duplicate provider types
        let result = config.validate();
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Duplicate provider type"));
        assert!(error_msg.contains("digitalocean"));
    }
}
