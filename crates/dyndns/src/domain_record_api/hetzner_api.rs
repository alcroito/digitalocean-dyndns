use color_eyre::eyre::{bail, eyre, Error, Result, WrapErr};
use reqwest::blocking::Client;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::RwLock;
use tracing::{debug, info, trace};

use crate::config::provider_config::{ProviderType, SecretProviderToken};
use crate::domain_record_api::DomainRecordApi;
use crate::types::DomainRecordToUpdate;

const HETZNER_API_BASE_URL: &str = "https://api.hetzner.cloud/v1";

#[derive(Deserialize, Debug)]
struct HetznerCloudZonesResponse {
    zones: Vec<HetznerZone>,
}

#[derive(Deserialize, Debug, Clone)]
struct HetznerZone {
    id: i64,
    name: String,
}

#[derive(Deserialize, Debug)]
struct HetznerRRSetsResponse {
    rrsets: Vec<HetznerRRSet>,
}

#[derive(Deserialize, Debug, Clone)]
struct HetznerRRSet {
    id: String,
    name: String,
    #[serde(rename = "type")]
    record_type: String,
    records: Vec<HetznerRecord>,
}

#[derive(Deserialize, Debug, Clone)]
struct HetznerRecord {
    value: String,
}

#[derive(Serialize, Debug)]
struct SetRecordsRequest {
    records: Vec<HetznerRecordInput>,
}

#[derive(Serialize, Debug)]
struct HetznerRecordInput {
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
}

#[derive(Deserialize, Debug)]
struct HetznerErrorResponse {
    error: HetznerError,
}

#[derive(Deserialize, Debug)]
struct HetznerError {
    code: String,
    message: String,
}

/// Helper to convert Hetzner `RRSet` to common format with zone context.
///
/// Hetzner returns full domain names in RRSet.name (e.g., "subdomain.domain.com"),
/// but we need to normalize to just the hostname part (e.g., "subdomain") for consistency
/// with other providers like `DigitalOcean`.
struct HetznerRRSetWithZone<'a> {
    rrset: HetznerRRSet,
    zone_name: &'a str,
}

impl<'a> HetznerRRSetWithZone<'a> {
    fn new(rrset: HetznerRRSet, zone_name: &'a str) -> Self {
        Self { rrset, zone_name }
    }
}

impl<'a> TryFrom<HetznerRRSetWithZone<'a>> for crate::types::DomainRecordCommon {
    type Error = Error;

    fn try_from(wrapper: HetznerRRSetWithZone<'a>) -> Result<Self, Self::Error> {
        let rrset = wrapper.rrset;
        let zone_name = wrapper.zone_name;

        // Extract first record value (typical for A/AAAA single-IP records)
        let ip_value = rrset
            .records
            .first()
            .ok_or_else(|| {
                eyre!(
                    "RRSet '{}' (type: {}) has no records",
                    rrset.name,
                    rrset.record_type
                )
            })?
            .value
            .clone();

        // Normalize the name field to contain only the hostname part
        // Hetzner returns full FQDNs like "subdomain.domain.com" or "@"
        // We need to convert to just "subdomain" or "@" for consistency
        let hostname_part = if rrset.name == "@" {
            // Apex domain
            "@".to_string()
        } else if rrset.name == zone_name {
            // Zone name itself means apex domain
            "@".to_string()
        } else if let Some(prefix) = rrset.name.strip_suffix(&format!(".{}", zone_name)) {
            // Strip zone suffix: "subdomain.domain.com" -> "subdomain"
            prefix.to_string()
        } else {
            // Fallback: use as-is if it doesn't match expected pattern
            rrset.name.clone()
        };

        Ok(Self {
            id: rrset.id,
            record_type: rrset.record_type,
            name: hostname_part,
            ip_value,
        })
    }
}

/// Handle Hetzner API error responses with optional custom error handling.
///
/// The closure can return `Some(error_msg)` to provide a custom error, or
/// `None` to fall back to the default error message.
fn handle_error_response_with_custom<F>(
    response: reqwest::blocking::Response,
    custom_error_handler: F,
) -> Result<reqwest::blocking::Response>
where
    F: FnOnce(String, String) -> Option<String>,
{
    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .unwrap_or_else(|_| "Unable to read error response".to_string());

        if let Ok(error_response) = serde_json::from_str::<HetznerErrorResponse>(&error_text) {
            // Try custom handler first
            if let Some(custom_msg) = custom_error_handler(
                error_response.error.code.clone(),
                error_response.error.message.clone(),
            ) {
                bail!("{}", custom_msg);
            }

            // Default error message
            bail!(
                "Hetzner API error ({}): {}",
                error_response.error.code,
                error_response.error.message
            );
        }

        bail!("HTTP {} error: {}", status, error_text);
    }

    Ok(response)
}

fn handle_error_response(
    response: reqwest::blocking::Response,
) -> Result<reqwest::blocking::Response> {
    handle_error_response_with_custom(response, |_, _| None)
}

pub struct HetznerApi {
    request_client: Client,
    token: SecretProviderToken,
    zone_cache: RwLock<HashMap<String, i64>>,
}

impl HetznerApi {
    pub fn new(token: SecretProviderToken) -> Self {
        Self {
            request_client: Client::new(),
            token,
            zone_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Find zone ID by name, with caching to avoid redundant API calls.
    fn find_zone_id(&self, zone_name: &str) -> Result<i64> {
        {
            let cache = self
                .zone_cache
                .read()
                .map_err(|e| eyre!("Failed to acquire read lock on zone cache: {}", e))?;
            if let Some(&zone_id) = cache.get(zone_name) {
                debug!("Zone ID for '{}' found in cache: {}", zone_name, zone_id);
                return Ok(zone_id);
            }
        }

        // Query API with name filter
        let url = format!("{}/zones?name={}", HETZNER_API_BASE_URL, zone_name);
        debug!("Fetching zone from API: {}", url);

        let response = self
            .request_client
            .get(&url)
            .bearer_auth(self.token.expose_secret().as_str())
            .send()
            .wrap_err("Failed to query Hetzner Cloud API for zones")?;

        let response = handle_error_response(response)?;

        let zones_response: HetznerCloudZonesResponse = response
            .json()
            .wrap_err("Failed to parse zones JSON response")?;

        let zone = zones_response
            .zones
            .into_iter()
            .find(|z| z.name == zone_name)
            .ok_or_else(|| {
                eyre!(
                    "Zone '{}' not found in Hetzner Cloud. Please create it first.",
                    zone_name
                )
            })?;

        let zone_id = zone.id;

        {
            let mut cache = self
                .zone_cache
                .write()
                .map_err(|e| eyre!("Failed to acquire write lock on zone cache: {}", e))?;
            cache.insert(zone_name.to_owned(), zone_id);
        }

        debug!("Found zone '{}' with ID: {}", zone_name, zone_id);
        Ok(zone_id)
    }
}

impl DomainRecordApi for HetznerApi {
    fn provider_name(&self) -> &'static str {
        "Hetzner Cloud"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Hetzner
    }

    fn get_domain_records(&self, domain_name: &str) -> Result<crate::types::DomainRecordsCommon> {
        let zone_id = self.find_zone_id(domain_name)?;

        let url = format!("{}/zones/{}/rrsets", HETZNER_API_BASE_URL, zone_id);
        debug!("Fetching RRSets from: {}", url);

        let response = self
            .request_client
            .get(&url)
            .bearer_auth(self.token.expose_secret().as_str())
            .send()
            .wrap_err("Failed to query Hetzner Cloud API for RRSets")?;

        let domain_name_owned = domain_name.to_string();
        let response = handle_error_response_with_custom(response, move |code, _message| {
            if code == "incorrect_zone_mode" {
                Some(format!(
                    "Zone '{}' is in secondary mode and cannot be managed via API",
                    domain_name_owned
                ))
            } else {
                None
            }
        })?;

        let rrsets_response: HetznerRRSetsResponse = response
            .json()
            .wrap_err("Failed to parse RRSets JSON response")?;

        debug!(
            "Found {} RRSets for zone '{}'",
            rrsets_response.rrsets.len(),
            domain_name
        );

        for rrset in &rrsets_response.rrsets {
            debug!(
                "Retrieved RRSet: id='{}', name='{}', type='{}'",
                rrset.id, rrset.name, rrset.record_type
            );
        }

        // Convert to common format for name normalization
        let converted_records: Result<Vec<_>, _> = rrsets_response
            .rrsets
            .into_iter()
            .map(|rrset| HetznerRRSetWithZone::new(rrset, domain_name).try_into())
            .collect();

        Ok(crate::types::DomainRecordsCommon {
            records: converted_records?,
        })
    }

    fn update_domain_ip(
        &self,
        domain_record_id: &str,
        record_to_update: &DomainRecordToUpdate,
        new_ip: &IpAddr,
    ) -> Result<()> {
        let zone_id = self.find_zone_id(&record_to_update.domain_name)?;
        let fqdn = record_to_update.fqdn();

        // Parse composite ID (format: "name/type", e.g., "www/A")
        let parts: Vec<&str> = domain_record_id.split('/').collect();
        let (rr_name, rr_type) = match parts.as_slice() {
            [name, record_type] => (*name, *record_type),
            _ => {
                bail!(
                    "Invalid RRSet ID format: '{}'. Expected 'name/type'",
                    domain_record_id
                );
            }
        };

        let url = format!(
            "{}/zones/{}/rrsets/{}/{}/actions/set_records",
            HETZNER_API_BASE_URL, zone_id, rr_name, rr_type
        );
        debug!("Updating RRSet at: {}", url);

        let payload = SetRecordsRequest {
            records: vec![HetznerRecordInput {
                value: new_ip.to_string(),
                comment: None,
            }],
        };

        let response = self
            .request_client
            .post(&url)
            .bearer_auth(self.token.expose_secret().as_str())
            .json(&payload)
            .send()
            .wrap_err(format!("Failed to update RRSet for: {}", fqdn))?;

        let domain_record_id_owned = domain_record_id.to_string();
        handle_error_response_with_custom(response, move |code, _message| match code.as_str() {
            "incorrect_zone_mode" => {
                Some("Zone is in secondary mode and cannot be updated".to_string())
            }
            "not_found" => Some(format!(
                "RRSet '{}' not found in zone",
                domain_record_id_owned
            )),
            _ => None,
        })?;

        info!("Successfully updated public IP for: {}", fqdn);
        Ok(())
    }
}

impl Drop for HetznerApi {
    fn drop(&mut self) {
        trace!("HetznerApi object destroyed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ip_fetcher::tests::MockIpFetcher;
    use crate::ip_fetcher::PublicIpFetcher;
    use color_eyre::eyre::{bail, WrapErr};
    struct MockApi {
        return_success: bool,
        zone_name: String,
    }

    impl MockApi {
        fn new(zone_name: &str) -> Self {
            Self {
                return_success: false,
                zone_name: zone_name.to_owned(),
            }
        }

        fn get_mock_domain_records_response() -> String {
            let path = [
                env!("CARGO_MANIFEST_DIR"),
                "tests/data/",
                "sample_hetzner_rrsets_response.json",
            ]
            .iter()
            .collect::<std::path::PathBuf>();
            std::fs::read_to_string(path).expect("Mock domain records not found")
        }

        fn parse_domain_records(s: &str) -> Result<HetznerRRSetsResponse> {
            let records: HetznerRRSetsResponse =
                serde_json::from_str(s).wrap_err("Failed to parse domain records JSON data")?;
            Ok(records)
        }
    }

    impl DomainRecordApi for MockApi {
        fn provider_name(&self) -> &'static str {
            "Mock Hetzner"
        }

        fn provider_type(&self) -> ProviderType {
            ProviderType::Hetzner
        }

        fn get_domain_records(
            &self,
            _domain_name: &str,
        ) -> Result<crate::types::DomainRecordsCommon> {
            let s = Self::get_mock_domain_records_response();
            let rrsets_response = Self::parse_domain_records(&s)?;

            // Convert to common format for name normalization
            let converted_records: Result<Vec<_>, _> = rrsets_response
                .rrsets
                .into_iter()
                .map(|rrset| HetznerRRSetWithZone::new(rrset, &self.zone_name).try_into())
                .collect();

            Ok(crate::types::DomainRecordsCommon {
                records: converted_records?,
            })
        }

        fn update_domain_ip(
            &self,
            _domain_record_id: &str,
            _record_to_update: &DomainRecordToUpdate,
            _new_ip: &IpAddr,
        ) -> Result<()> {
            if self.return_success {
                Ok(())
            } else {
                bail!("Failed to update domain ip")
            }
        }
    }

    #[test]
    fn test_basic() {
        use crate::updater::{get_record_to_update, should_update_domain_ip};

        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "config.toml",
                r#"
domain_root = "site.com"
subdomain_to_update = "home"

[[providers]]
provider = "hetzner"
token = "123"
            "#,
            )?;

            let config_builder = crate::config::app_config_builder::AppConfigBuilder::new(
                None,
                Some("config.toml".to_owned()),
            )
            .expect("Failed to create config builder");
            let config = config_builder.build().expect("failed to parse config");
            let ip_fetcher = MockIpFetcher::default();
            let public_ips = ip_fetcher.fetch_public_ips(true, true).unwrap();
            let domain_name = &config.domains.domains.first().expect("no domain").name;
            let updater = MockApi::new(domain_name);
            let hostname_part = &config
                .domains
                .domains
                .first()
                .expect("no domain")
                .records
                .first()
                .expect("no record")
                .name;
            let record_type = "A";
            let record_to_update =
                DomainRecordToUpdate::new(domain_name, hostname_part, record_type, None);

            let records = updater.get_domain_records(domain_name).unwrap();
            let domain_record = get_record_to_update(&records, &record_to_update).unwrap();
            let (ip_addr, _ip_kind) = public_ips.to_ip_addr_from_any();
            let should_update = should_update_domain_ip(&ip_addr, domain_record);

            assert!(should_update);

            let result = updater.update_domain_ip(&domain_record.id, &record_to_update, &ip_addr);
            assert!(result.is_err());

            Ok(())
        });
    }
}
