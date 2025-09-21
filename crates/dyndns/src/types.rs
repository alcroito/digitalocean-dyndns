use color_eyre::eyre::{eyre, Error};
use std::{fmt::Display, net::IpAddr};
use tracing::Level;

use crate::config::app_config::UpdateInterval;
use crate::config::provider_config::{ProviderType, SecretProviderToken};

/// Controls the default behavior for domain records without an explicit providers list.
#[derive(Debug, Clone, Copy)]
pub enum ProvidersMissingBehavior {
    UpdateForAllProviders,
    DoNothing,
}

impl From<bool> for ProvidersMissingBehavior {
    fn from(value: bool) -> Self {
        if value {
            ProvidersMissingBehavior::UpdateForAllProviders
        } else {
            ProvidersMissingBehavior::DoNothing
        }
    }
}

impl From<ProvidersMissingBehavior> for bool {
    fn from(value: ProvidersMissingBehavior) -> Self {
        matches!(value, ProvidersMissingBehavior::UpdateForAllProviders)
    }
}

/// Provider-agnostic common domain record representation.
///
/// This type unifies domain records from different DNS providers by:
/// - Using String for IDs
/// - Normalizing the `name` field to contain only the hostname part (e.g., "www", "@", "home")
///   regardless of what format the provider returns (some return FQDNs, others just hostnames)
/// - Using a single field name (`ip_value`) for the IP address regardless of provider
#[derive(Debug, Clone)]
pub struct DomainRecordCommon {
    /// Unique identifier for the record (String to support all providers)
    pub id: String,
    /// DNS record type (A, AAAA, CNAME, etc.)
    pub record_type: String,
    /// Hostname part of the record (e.g., "www", "@", "home")
    /// NOTE: This must be normalized to hostname-only format by provider implementations
    pub name: String,
    pub ip_value: String,
}

#[derive(Debug, Clone)]
pub struct DomainRecordsCommon {
    pub records: Vec<DomainRecordCommon>,
}

/// Cache mapping domain names to their DNS records fetched from provider APIs.
///
/// Avoids redundant API calls.
pub type DomainRecordCache = std::collections::HashMap<String, DomainRecordsCommon>;

#[derive(Debug)]
pub struct DomainRecordToUpdate {
    pub domain_name: String,
    pub hostname_part: String,
    pub record_type: String,
    /// Optional list of providers to update this record on.
    /// - `None`: updates on ALL configured providers (default behavior)
    /// - `Some(vec![])`: updates on NO providers (explicitly disabled)
    /// - `Some(vec![...])`: updates only on specified providers
    pub providers: Option<Vec<ProviderType>>,
}

impl DomainRecordToUpdate {
    pub fn new(
        domain_name: &str,
        hostname_part: &str,
        record_type: &str,
        providers: Option<Vec<ProviderType>>,
    ) -> Self {
        DomainRecordToUpdate {
            domain_name: domain_name.to_owned(),
            hostname_part: hostname_part.to_owned(),
            record_type: record_type.to_owned(),
            providers,
        }
    }

    pub fn fqdn(&self) -> String {
        if self.hostname_part == "@" {
            self.domain_name.clone()
        } else {
            format!("{}.{}", self.hostname_part, self.domain_name)
        }
    }

    /// Check if this record should be updated on the given provider
    pub fn should_update_on(
        &self,
        provider: ProviderType,
        update_all_providers_by_default: ProvidersMissingBehavior,
    ) -> bool {
        match &self.providers {
            // If providers is None, behavior depends on global config flag
            None => update_all_providers_by_default.into(),
            // If providers is Some with empty vec, don't update on any provider
            Some(vec) if vec.is_empty() => false,
            // Otherwise, check if the provider is in the list
            Some(providers) => providers.contains(&provider),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct IpAddrV4AndV6 {
    pub ipv4: Option<std::net::Ipv4Addr>,
    pub ipv6: Option<std::net::Ipv6Addr>,
}

#[derive(Debug, Copy, Clone)]
pub enum IpAddrKind {
    V4,
    V6,
}

impl Display for IpAddrV4AndV6 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ipv4: {:?} ipv6: {:?}", self.ipv4, self.ipv6)
    }
}

pub struct DisplayIpAddrV4AndV6Pretty<'a>(pub &'a IpAddrV4AndV6);

impl Display for DisplayIpAddrV4AndV6Pretty<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.has_both() {
            writeln!(f, "Public IPs are:")?;
            writeln!(f, "  ipv4: {}", self.0.to_ip_addr_from_ipv4())?;
            writeln!(f, "  ipv6: {}", self.0.to_ip_addr_from_ipv6())?;
        } else if self.0.has_any() {
            write!(f, "Public IP is: ")?;
            if self.0.has_ipv4() {
                writeln!(f, "{}", self.0.to_ip_addr_from_ipv4())?;
            }
            if self.0.has_ipv6() {
                writeln!(f, "{}", self.0.to_ip_addr_from_ipv6())?;
            }
        } else {
            writeln!(f, "No public IP")?;
        }
        Ok(())
    }
}

impl IpAddrV4AndV6 {
    pub fn has_none(&self) -> bool {
        self.ipv4.is_none() && self.ipv6.is_none()
    }

    pub fn has_any(&self) -> bool {
        self.ipv4.is_some() || self.ipv6.is_some()
    }

    pub fn has_both(&self) -> bool {
        self.ipv4.is_some() && self.ipv6.is_some()
    }

    pub fn has_ipv4(&self) -> bool {
        self.ipv4.is_some()
    }

    pub fn has_ipv6(&self) -> bool {
        self.ipv6.is_some()
    }

    pub fn to_ip_addr_from_ipv4(&self) -> IpAddr {
        IpAddr::V4(self.ipv4.expect("Does not contain a ipv4 address"))
    }

    pub fn to_ip_addr_from_ipv6(&self) -> IpAddr {
        IpAddr::V6(self.ipv6.expect("Does not contain a ipv4 address"))
    }

    pub fn to_ipv4_string(&self) -> Option<String> {
        self.ipv4.map(|ip| ip.to_string())
    }

    pub fn to_ipv6_string(&self) -> Option<String> {
        self.ipv6.map(|ip| ip.to_string())
    }

    pub fn to_ip_addr_from_any(&self) -> (IpAddr, IpAddrKind) {
        if self.has_ipv4() {
            (self.to_ip_addr_from_ipv4(), IpAddrKind::V4)
        } else {
            (self.to_ip_addr_from_ipv6(), IpAddrKind::V6)
        }
    }
}

impl From<IpAddr> for IpAddrV4AndV6 {
    fn from(value: IpAddr) -> Self {
        match value {
            IpAddr::V4(ip) => IpAddrV4AndV6 {
                ipv4: Some(ip),
                ..Default::default()
            },
            IpAddr::V6(ip) => IpAddrV4AndV6 {
                ipv6: Some(ip),
                ..Default::default()
            },
        }
    }
}

pub trait ValueFromStr: Sized {
    type Err;
    fn from_str(s: &str) -> Result<Self, Self::Err>;
}

// This doesn't work with the following error:
//
// conflicting implementations of trait `types::ValueFromStr` for type 'x'
// upstream crates may add a new impl of trait `std::str::FromStr` for type 'x' in future versions
//
// Apparently the recommended workaround is to use macros and be explicit
// about types.
// impl<T> ValueFromStr for T
// where
//     T: std::str::FromStr,
// {
//     type Err = T::Err;
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         s.parse::<T>()
//     }
// }

macro_rules! impl_value_from_str {
    ( $($t:ty),* ) => {
        $( impl ValueFromStr for $t {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self, Self::Err>
            {
                match s.parse::<$t>() {
                    Ok(val) => Ok(val),
                    Err(e) => Err(e.into()),
                }
            }
        })*
    }
}

impl_value_from_str! { String, bool, UpdateInterval, Level, u16 }

pub trait ValueFromBool: Sized {
    type Err;
    fn from_bool(b: bool) -> Result<Self, Self::Err>;
}

impl ValueFromBool for bool {
    type Err = Error;
    fn from_bool(b: bool) -> Result<Self, Self::Err> {
        Ok(b)
    }
}

macro_rules! impl_value_from_bool_as_error {
    ( $($t:ty),* ) => {
        $( impl ValueFromBool for $t {
            type Err = Error;
            fn from_bool(_b: bool) -> Result<Self, Self::Err>
            {
                 Err(eyre!("Can not convert from bool to target type"))
            }
        })*
    }
}

impl_value_from_bool_as_error! { String, UpdateInterval, Level, SecretProviderToken, u16 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_record_to_update_should_update_on_none() {
        let record = DomainRecordToUpdate::new("example.com", "test", "A", None);

        // Should update on all providers when providers is None
        const BEHAVIOR: ProvidersMissingBehavior = ProvidersMissingBehavior::UpdateForAllProviders;
        assert!(record.should_update_on(ProviderType::DigitalOcean, BEHAVIOR));
        assert!(record.should_update_on(ProviderType::Hetzner, BEHAVIOR));

        // Should NOT update on any providers when providers is None
        const BEHAVIOR_NO: ProvidersMissingBehavior = ProvidersMissingBehavior::DoNothing;
        assert!(!record.should_update_on(ProviderType::DigitalOcean, BEHAVIOR_NO));
        assert!(!record.should_update_on(ProviderType::Hetzner, BEHAVIOR_NO));
    }

    #[test]
    fn test_domain_record_to_update_should_update_on_empty() {
        let record = DomainRecordToUpdate::new("example.com", "test", "A", Some(vec![]));

        // Should NOT update on any providers when providers list is empty (explicitly disabled)
        // regardless of the global flag
        const BEHAVIOR_YES: ProvidersMissingBehavior =
            ProvidersMissingBehavior::UpdateForAllProviders;
        assert!(!record.should_update_on(ProviderType::DigitalOcean, BEHAVIOR_YES));
        assert!(!record.should_update_on(ProviderType::Hetzner, BEHAVIOR_YES));

        const BEHAVIOR_NO: ProvidersMissingBehavior = ProvidersMissingBehavior::DoNothing;
        assert!(!record.should_update_on(ProviderType::DigitalOcean, BEHAVIOR_NO));
        assert!(!record.should_update_on(ProviderType::Hetzner, BEHAVIOR_NO));
    }

    #[test]
    fn test_domain_record_to_update_should_update_on_specific_provider() {
        let record = DomainRecordToUpdate::new(
            "example.com",
            "test",
            "A",
            Some(vec![ProviderType::DigitalOcean]),
        );

        // Should only update on DigitalOcean regardless of the global flag
        const BEHAVIOR_YES: ProvidersMissingBehavior =
            ProvidersMissingBehavior::UpdateForAllProviders;
        assert!(record.should_update_on(ProviderType::DigitalOcean, BEHAVIOR_YES));
        assert!(!record.should_update_on(ProviderType::Hetzner, BEHAVIOR_YES));

        const BEHAVIOR_NO: ProvidersMissingBehavior = ProvidersMissingBehavior::DoNothing;
        assert!(record.should_update_on(ProviderType::DigitalOcean, BEHAVIOR_NO));
        assert!(!record.should_update_on(ProviderType::Hetzner, BEHAVIOR_NO));
    }

    #[test]
    fn test_domain_record_to_update_should_update_on_multiple_providers() {
        let record = DomainRecordToUpdate::new(
            "example.com",
            "test",
            "A",
            Some(vec![ProviderType::DigitalOcean, ProviderType::Hetzner]),
        );

        // Should update on both providers regardless of the global flag
        const BEHAVIOR_YES: ProvidersMissingBehavior =
            ProvidersMissingBehavior::UpdateForAllProviders;
        assert!(record.should_update_on(ProviderType::DigitalOcean, BEHAVIOR_YES));
        assert!(record.should_update_on(ProviderType::Hetzner, BEHAVIOR_YES));

        const BEHAVIOR_NO: ProvidersMissingBehavior = ProvidersMissingBehavior::DoNothing;
        assert!(record.should_update_on(ProviderType::DigitalOcean, BEHAVIOR_NO));
        assert!(record.should_update_on(ProviderType::Hetzner, BEHAVIOR_NO));
    }

    #[test]
    fn test_domain_record_to_update_fqdn() {
        // Test with regular subdomain
        let record1 = DomainRecordToUpdate::new("example.com", "www", "A", None);
        assert_eq!(record1.fqdn(), "www.example.com");

        // Test with @ (root domain)
        let record2 = DomainRecordToUpdate::new("example.com", "@", "A", None);
        assert_eq!(record2.fqdn(), "example.com");

        // Test with nested subdomain
        let record3 = DomainRecordToUpdate::new("example.com", "api.v1", "A", None);
        assert_eq!(record3.fqdn(), "api.v1.example.com");
    }
}
