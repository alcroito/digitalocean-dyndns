use color_eyre::eyre::{bail, Error, Result};
use secrecy::{zeroize::Zeroize, SecretBox};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    #[serde(alias = "digitalocean", alias = "digital_ocean")]
    DigitalOcean,
    Hetzner,
}

impl ProviderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderType::DigitalOcean => "digitalocean",
            ProviderType::Hetzner => "hetzner",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ProviderToken(String);

pub type SecretProviderToken = SecretBox<ProviderToken>;

impl ProviderToken {
    pub fn new(token: String) -> Self {
        Self(token)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
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
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ProviderToken(s.to_owned()))
    }
}

pub fn parse_secret_token(s: &str) -> Result<SecretProviderToken, Error> {
    Ok(SecretBox::new(Box::new(s.parse::<ProviderToken>()?)))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider: ProviderType,
    pub token: SecretProviderToken,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersConfig {
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
}

impl ProvidersConfig {
    pub fn get(&self, provider_type: ProviderType) -> Option<&ProviderConfig> {
        self.providers.iter().find(|p| p.provider == provider_type)
    }

    pub fn has_provider(&self, provider_type: ProviderType) -> bool {
        self.get(provider_type).is_some()
    }

    pub fn provider_types(&self) -> Vec<ProviderType> {
        self.providers.iter().map(|p| p.provider).collect()
    }

    pub fn validate(&self) -> Result<()> {
        self.validate_not_empty()?;
        self.validate_no_duplicates()?;

        Ok(())
    }

    fn validate_no_duplicates(&self) -> Result<()> {
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        for provider_config in &self.providers {
            if !seen.insert(provider_config.provider) {
                bail!(
                    "Duplicate provider type '{}' found. Each provider type can only be configured once.",
                    provider_config.provider.as_str()
                );
            }
        }
        Ok(())
    }

    pub fn validate_not_empty(&self) -> Result<()> {
        if self.providers.is_empty() {
            bail!("At least one DNS provider must be configured.");
        }
        Ok(())
    }
}
