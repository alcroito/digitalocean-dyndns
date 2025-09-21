use color_eyre::eyre::{Context, Result};
use secrecy::ExposeSecret;
use std::net::IpAddr;
use std::str::FromStr;

use crate::config::provider_config::{ProviderConfig, ProviderType};
use crate::digital_ocean_token::{DigitalOceanToken, SecretDigitalOceanToken};
use crate::hetzner_token::{HetznerToken, SecretHetznerToken};
use crate::types::{DomainRecordToUpdate, DomainRecordsCommon};

/// Trait for DNS provider implementations.
///
/// This trait abstracts over different DNS providers (`DigitalOcean`, Hetzner, etc.)
/// using provider-agnostic common types. Implementors should convert their
/// provider-specific API types to the common types defined in `types`.
pub trait DomainRecordApi {
    /// Returns the name of the DNS provider (e.g., `DigitalOcean`, `Hetzner`).
    fn provider_name(&self) -> &'static str;

    /// Returns the provider type enum for this provider.
    fn provider_type(&self) -> ProviderType;

    /// Fetch all DNS records for a given domain.
    ///
    /// Returns records in the provider-agnostic common format.
    fn get_domain_records(&self, domain_name: &str) -> Result<DomainRecordsCommon>;

    /// Update a specific domain record with a new IP address.
    ///
    /// # Arguments
    /// * `domain_record_id` - The record identifier (String to support all providers)
    /// * `record_to_update` - Details about the record being updated
    /// * `new_ip` - The new IP address to set
    fn update_domain_ip(
        &self,
        domain_record_id: &str,
        record_to_update: &DomainRecordToUpdate,
        new_ip: &IpAddr,
    ) -> Result<()>;
}

/// Factory function to create a provider from configuration.
///
/// Converts the generic `SecretProviderToken` to the provider-specific token type
/// and instantiates the appropriate provider implementation.
pub fn create_provider(config: &ProviderConfig) -> Result<Box<dyn DomainRecordApi + Send>> {
    match config.provider {
        ProviderType::DigitalOcean => {
            // Convert SecretProviderToken to SecretDigitalOceanToken
            let token_str = config.token.expose_secret().as_str();
            let token = DigitalOceanToken::from_str(token_str).context(format!(
                "Failed to parse DigitalOcean token for provider '{}'. \
                 Ensure the token is a valid DigitalOcean API token.",
                config.provider.as_str()
            ))?;
            let secret_token = SecretDigitalOceanToken::new(Box::new(token));
            Ok(Box::new(digital_ocean_api::DigitalOceanApi::new(
                secret_token,
            )))
        }
        ProviderType::Hetzner => {
            // Convert SecretProviderToken to SecretHetznerToken
            let token_str = config.token.expose_secret().as_str();
            let token = HetznerToken::from_str(token_str).context(format!(
                "Failed to parse Hetzner token for provider '{}'. \
                 Ensure the token is a valid Hetzner DNS API token.",
                config.provider.as_str()
            ))?;
            let secret_token = SecretHetznerToken::new(Box::new(token));
            Ok(Box::new(hetzner_api::HetznerApi::new(secret_token)))
        }
    }
}

pub mod digital_ocean_api;
pub mod hetzner_api;
