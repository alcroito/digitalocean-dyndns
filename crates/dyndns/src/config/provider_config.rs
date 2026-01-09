//! Provider configuration types for managing DNS provider settings.
//!
//! This module provides a scalable way to configure multiple DNS providers with
//! provider-specific options. It replaces the legacy top-level token configuration
//! with a more structured approach.
//!
//! # Example
//!
//! ```toml
//! [[providers]]
//! provider = "digitalocean"
//! token = "your_api_token_here"
//!
//! [[providers]]
//! provider = "hetzner"
//! token = "your_hetzner_token"
//! ```

use color_eyre::eyre::{bail, Result};
use secrecy::{zeroize::Zeroize, SecretBox};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Identifies a supported DNS provider.
///
/// This enum is used throughout the configuration system to specify which DNS
/// provider should be used for updating records.
///
/// # Serialization
///
/// When serialized to TOML/JSON, provider types use lowercase names:
/// - `DigitalOcean` -> `"digitalocean"`
/// - `Hetzner` -> `"hetzner"`
///
/// The `DigitalOcean` variant also accepts the alias `"digital_ocean"` for
/// backwards compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    /// `DigitalOcean` DNS provider
    #[serde(alias = "digitalocean", alias = "digital_ocean")]
    DigitalOcean,
    /// Hetzner DNS provider
    Hetzner,
}

impl ProviderType {
    /// Returns the lowercase string representation of the provider type.
    ///
    /// This is useful for display purposes and cache keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use do_ddns::config::provider_config::ProviderType;
    ///
    /// assert_eq!(ProviderType::DigitalOcean.as_str(), "digitalocean");
    /// assert_eq!(ProviderType::Hetzner.as_str(), "hetzner");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderType::DigitalOcean => "digitalocean",
            ProviderType::Hetzner => "hetzner",
        }
    }
}

/// Generic provider token wrapper for secure handling via the `secrecy` crate.
///
/// This type wraps a token string and implements the necessary traits for
/// secure storage, serialization, and zeroization when dropped.
#[derive(Clone, Serialize, Deserialize)]
pub struct ProviderToken(String);

/// A type alias for a securely stored provider token.
///
/// The token is wrapped in `SecretBox` from the `secrecy` crate, which ensures
/// the token value is not accidentally exposed in logs or error messages.
pub type SecretProviderToken = SecretBox<ProviderToken>;

impl ProviderToken {
    /// Returns the token as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Creates a new provider token from a string.
    pub fn new(token: String) -> Self {
        Self(token)
    }
}

impl Zeroize for ProviderToken {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl secrecy::CloneableSecret for ProviderToken {}
impl secrecy::SerializableSecret for ProviderToken {}

impl std::str::FromStr for ProviderToken {
    type Err = color_eyre::eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ProviderToken(s.to_owned()))
    }
}

/// Configuration for a single DNS provider.
///
/// This struct contains all the information needed to instantiate and use
/// a DNS provider, including the provider type, authentication token, and
/// any provider-specific options.
///
/// # Example
///
/// ```toml
/// [[providers]]
/// provider = "digitalocean"
/// token = "your_api_token"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// The type of DNS provider (e.g., `DigitalOcean`, Hetzner)
    pub provider: ProviderType,

    /// The authentication token for the provider.
    ///
    /// This is securely stored using the `secrecy` crate to prevent
    /// accidental exposure in logs or error messages.
    pub token: SecretProviderToken,

    /// Provider-specific configuration options.
    ///
    /// This extensible `HashMap` allows each provider to define its own
    /// additional configuration parameters. For example, Cloudflare might
    /// require a `zone_id` option.
    ///
    /// # Example
    ///
    /// ```toml
    /// [[providers]]
    /// provider = "cloudflare"
    /// token = "token"
    /// zone_id = "abc123"
    /// ```
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

/// Collection of provider configurations.
///
/// This struct holds all configured DNS providers and provides helper methods
/// for querying and validating the configuration.
///
/// # Example
///
/// ```toml
/// [[providers]]
/// provider = "digitalocean"
/// token = "token1"
///
/// [[providers]]
/// provider = "hetzner"
/// token = "token2"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersConfig {
    /// List of configured providers
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
}

impl ProvidersConfig {
    /// Returns the configuration for a specific provider type, if configured.
    ///
    /// # Examples
    ///
    /// ```
    /// use do_ddns::config::provider_config::{ProvidersConfig, ProviderType};
    ///
    /// let config = ProvidersConfig::default();
    /// assert!(config.get(ProviderType::DigitalOcean).is_none());
    /// ```
    pub fn get(&self, provider_type: ProviderType) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.provider == provider_type)
    }

    /// Checks if a provider of the given type is configured.
    ///
    /// # Examples
    ///
    /// ```
    /// use do_ddns::config::provider_config::{ProvidersConfig, ProviderType};
    ///
    /// let config = ProvidersConfig::default();
    /// assert!(!config.has_provider(ProviderType::Hetzner));
    /// ```
    pub fn has_provider(&self, provider_type: ProviderType) -> bool {
        self.get(provider_type).is_some()
    }

    /// Returns a list of all configured provider types.
    ///
    /// # Examples
    ///
    /// ```
    /// use do_ddns::config::provider_config::ProvidersConfig;
    ///
    /// let config = ProvidersConfig::default();
    /// assert_eq!(config.provider_types().len(), 0);
    /// ```
    pub fn provider_types(&self) -> Vec<ProviderType> {
        self.providers.iter().map(|p| p.provider).collect()
    }

    /// Validates that at least one provider is configured.
    ///
    /// Returns an error if no providers are configured, as the application
    /// requires at least one DNS provider to function.
    ///
    /// # Errors
    ///
    /// Returns an error if the `providers` list is empty.
    pub fn validate(&self) -> Result<()> {
        if self.providers.is_empty() {
            bail!("At least one DNS provider must be configured.");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[test]
    fn test_provider_type_as_str() {
        assert_eq!(ProviderType::DigitalOcean.as_str(), "digitalocean");
        assert_eq!(ProviderType::Hetzner.as_str(), "hetzner");
    }

    #[test]
    fn test_provider_type_deserialization() {
        // Test lowercase
        let json = r#""digitalocean""#;
        let provider: ProviderType = serde_json::from_str(json).unwrap();
        assert_eq!(provider, ProviderType::DigitalOcean);

        let json = r#""hetzner""#;
        let provider: ProviderType = serde_json::from_str(json).unwrap();
        assert_eq!(provider, ProviderType::Hetzner);
    }

    #[test]
    fn test_provider_type_aliases() {
        // Test aliases for DigitalOcean
        let json = r#""digitalocean""#;
        let provider: ProviderType = serde_json::from_str(json).unwrap();
        assert_eq!(provider, ProviderType::DigitalOcean);

        let json = r#""digital_ocean""#;
        let provider: ProviderType = serde_json::from_str(json).unwrap();
        assert_eq!(provider, ProviderType::DigitalOcean);
    }

    #[test]
    fn test_provider_type_serialization() {
        let provider = ProviderType::DigitalOcean;
        let json = serde_json::to_string(&provider).unwrap();
        assert_eq!(json, r#""digitalocean""#);

        let provider = ProviderType::Hetzner;
        let json = serde_json::to_string(&provider).unwrap();
        assert_eq!(json, r#""hetzner""#);
    }

    #[test]
    fn test_invalid_provider_type() {
        let json = r#""invalid_provider""#;
        let result: Result<ProviderType, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_config_deserialization() {
        let toml = r#"
            provider = "digitalocean"
            token = "test_token_123"
        "#;
        let config: ProviderConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.provider, ProviderType::DigitalOcean);
        assert_eq!(config.token.expose_secret().as_str(), "test_token_123");
        assert!(config.options.is_empty());
    }

    #[test]
    fn test_provider_config_with_options() {
        let toml = r#"
            provider = "hetzner"
            token = "hetzner_token"
            
            [options]
            timeout = 30
            rate_limit = 100
        "#;
        let config: ProviderConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.provider, ProviderType::Hetzner);
        assert_eq!(config.token.expose_secret().as_str(), "hetzner_token");
        assert_eq!(config.options.len(), 2);
        assert_eq!(config.options["timeout"], serde_json::json!(30));
        assert_eq!(config.options["rate_limit"], serde_json::json!(100));
    }

    #[test]
    fn test_providers_config_deserialization() {
        let toml = r#"
            [[providers]]
            provider = "digitalocean"
            token = "do_token"
            
            [[providers]]
            provider = "hetzner"
            token = "hz_token"
        "#;
        let config: ProvidersConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.providers.len(), 2);
        assert_eq!(config.providers[0].provider, ProviderType::DigitalOcean);
        assert_eq!(config.providers[1].provider, ProviderType::Hetzner);
    }

    #[test]
    fn test_providers_config_get() {
        let toml = r#"
            [[providers]]
            provider = "digitalocean"
            token = "do_token"
        "#;
        let config: ProvidersConfig = toml::from_str(toml).unwrap();

        assert!(config.get(ProviderType::DigitalOcean).is_some());
        assert!(config.get(ProviderType::Hetzner).is_none());
    }

    #[test]
    fn test_providers_config_has_provider() {
        let toml = r#"
            [[providers]]
            provider = "hetzner"
            token = "hz_token"
        "#;
        let config: ProvidersConfig = toml::from_str(toml).unwrap();

        assert!(!config.has_provider(ProviderType::DigitalOcean));
        assert!(config.has_provider(ProviderType::Hetzner));
    }

    #[test]
    fn test_providers_config_provider_types() {
        let toml = r#"
            [[providers]]
            provider = "digitalocean"
            token = "do_token"
            
            [[providers]]
            provider = "hetzner"
            token = "hz_token"
        "#;
        let config: ProvidersConfig = toml::from_str(toml).unwrap();
        let types = config.provider_types();

        assert_eq!(types.len(), 2);
        assert!(types.contains(&ProviderType::DigitalOcean));
        assert!(types.contains(&ProviderType::Hetzner));
    }

    #[test]
    fn test_providers_config_validate_empty() {
        let config = ProvidersConfig::default();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("At least one DNS provider must be configured"));
    }

    #[test]
    fn test_providers_config_validate_success() {
        let toml = r#"
            [[providers]]
            provider = "digitalocean"
            token = "do_token"
        "#;
        let config: ProvidersConfig = toml::from_str(toml).unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_provider_type_hash_equality() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ProviderType::DigitalOcean);
        set.insert(ProviderType::Hetzner);
        set.insert(ProviderType::DigitalOcean); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&ProviderType::DigitalOcean));
        assert!(set.contains(&ProviderType::Hetzner));
    }

    #[test]
    fn test_empty_providers_config_deserialization() {
        let toml = r#""#;
        let config: ProvidersConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.providers.len(), 0);
    }

    #[test]
    fn test_provider_config_missing_token() {
        let toml = r#"
            provider = "digitalocean"
        "#;
        let result: Result<ProviderConfig, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_config_missing_provider() {
        let toml = r#"
            token = "test_token"
        "#;
        let result: Result<ProviderConfig, _> = toml::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_config_complex_options() {
        let toml = r#"
            provider = "digitalocean"
            token = "token"
            
            [options]
            string_value = "test"
            number_value = 42
            bool_value = true
            array_value = [1, 2, 3]
        "#;
        let config: ProviderConfig = toml::from_str(toml).unwrap();

        assert_eq!(config.options["string_value"], serde_json::json!("test"));
        assert_eq!(config.options["number_value"], serde_json::json!(42));
        assert_eq!(config.options["bool_value"], serde_json::json!(true));
        assert_eq!(config.options["array_value"], serde_json::json!([1, 2, 3]));
    }
}
