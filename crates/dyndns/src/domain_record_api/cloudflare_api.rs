use color_eyre::eyre::{bail, eyre, Error, Result, WrapErr};
use reqwest::blocking::Client;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::RwLock;
use tracing::{debug, info};

use crate::config::provider_config::{ProviderType, SecretProviderToken};
use crate::domain_record_api::{format_record_value, strip_record_value, DomainRecordApi};
use crate::types::DomainRecordToUpdate;

const CLOUDFLARE_API_BASE_URL: &str = "https://api.cloudflare.com/client/v4";

#[derive(Deserialize, Debug)]
struct CloudflareZonesResponse {
    result: Vec<CloudflareZone>,
    success: bool,
}

#[derive(Deserialize, Debug)]
struct CloudflareZone {
    id: String,
    name: String,
}

#[derive(Deserialize, Debug)]
struct CloudflareDnsRecordsResponse {
    result: Vec<CloudflareDnsRecord>,
    success: bool,
}

#[derive(Deserialize, Debug, Clone)]
struct CloudflareDnsRecord {
    id: String,
    #[serde(rename = "type")]
    record_type: String,
    name: String,    // FQDN, e.g. "home.site.com"
    content: String, // IP address
    ttl: u32,
    proxied: bool,
}

#[derive(Serialize, Debug)]
struct CloudflareUpdateRecordRequest {
    #[serde(rename = "type")]
    record_type: String,
    name: String,    // FQDN
    content: String, // new IP
    ttl: u32,
    proxied: bool,
}

#[derive(Deserialize, Debug)]
struct CloudflareErrorResponse {
    errors: Vec<CloudflareError>,
}

#[derive(Deserialize, Debug)]
struct CloudflareError {
    code: u32,
    message: String,
}

/// Wraps a `CloudflareDnsRecord` with its zone name so the name can be normalized.
struct CloudflareDnsRecordWithZone<'a> {
    record: CloudflareDnsRecord,
    zone_name: &'a str,
}

impl<'a> CloudflareDnsRecordWithZone<'a> {
    fn new(record: CloudflareDnsRecord, zone_name: &'a str) -> Self {
        Self { record, zone_name }
    }
}

impl<'a> TryFrom<CloudflareDnsRecordWithZone<'a>> for crate::types::DomainRecordCommon {
    type Error = Error;

    fn try_from(wrapper: CloudflareDnsRecordWithZone<'a>) -> Result<Self, Self::Error> {
        let record = wrapper.record;
        let zone_name = wrapper.zone_name;

        // Normalize the name field to contain only the hostname part.
        // We need to convert to just "subdomain" or "@" for consistency
        let hostname_part = if record.name == zone_name {
            "@".to_string()
        } else if let Some(prefix) = record.name.strip_suffix(&format!(".{}", zone_name)) {
            prefix.to_string()
        } else {
            record.name.clone()
        };

        // Encode proxy status and TTL into the composite ID so they can be
        // preserved when update_domain_ip is called later.
        let composite_id = format!("{}/{}/{}", record.id, record.ttl, record.proxied);

        Ok(Self {
            id: composite_id,
            record_type: record.record_type,
            name: hostname_part,
            ip_value: strip_record_value(&record.content).to_string(),
        })
    }
}

/// Handle Cloudflare API error responses.
fn handle_error_response(
    response: reqwest::blocking::Response,
) -> Result<reqwest::blocking::Response> {
    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .unwrap_or_else(|_| "Unable to read error response".to_string());

        if let Ok(error_response) = serde_json::from_str::<CloudflareErrorResponse>(&error_text) {
            if let Some(error) = error_response.errors.first() {
                bail!(
                    "Cloudflare API error (code {}): {}",
                    error.code,
                    error.message
                );
            }
        }

        bail!("HTTP {} error: {}", status, error_text);
    }

    Ok(response)
}

pub struct CloudflareApi {
    request_client: Client,
    token: SecretProviderToken,
    zone_cache: RwLock<HashMap<String, String>>, // domain_name -> zone_id
}

impl CloudflareApi {
    pub fn new(token: SecretProviderToken) -> Self {
        Self {
            request_client: Client::new(),
            token,
            zone_cache: RwLock::new(HashMap::new()),
        }
    }

    fn find_zone_id(&self, domain_name: &str) -> Result<String> {
        {
            let cache = self
                .zone_cache
                .read()
                .map_err(|e| eyre!("Failed to acquire read lock on zone cache: {}", e))?;
            if let Some(zone_id) = cache.get(domain_name) {
                debug!("Zone ID for '{}' found in cache: {}", domain_name, zone_id);
                return Ok(zone_id.clone());
            }
        }

        let url = format!("{}/zones?name={}", CLOUDFLARE_API_BASE_URL, domain_name);
        debug!("Fetching Cloudflare zone for domain: {}", domain_name);

        let response = self
            .request_client
            .get(&url)
            .bearer_auth(self.token.expose_secret().as_str())
            .send()
            .wrap_err("Failed to query Cloudflare API for zones")?;

        let response = handle_error_response(response)?;

        let zones_response: CloudflareZonesResponse = response
            .json()
            .wrap_err("Failed to parse Cloudflare zones JSON response")?;

        if !zones_response.success {
            bail!(
                "Cloudflare API returned success=false for zone lookup of '{}'",
                domain_name
            );
        }

        let zone = zones_response
            .result
            .into_iter()
            .find(|z| z.name == domain_name)
            .ok_or_else(|| {
                eyre!(
                    "Zone '{}' not found in Cloudflare. Please create it first.",
                    domain_name
                )
            })?;

        let zone_id = zone.id;

        {
            let mut cache = self
                .zone_cache
                .write()
                .map_err(|e| eyre!("Failed to acquire write lock on zone cache: {}", e))?;
            cache.insert(domain_name.to_owned(), zone_id.clone());
        }

        debug!(
            "Found Cloudflare zone '{}' with ID: {}",
            domain_name, zone_id
        );
        Ok(zone_id)
    }
}

impl DomainRecordApi for CloudflareApi {
    fn provider_name(&self) -> &'static str {
        "Cloudflare"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Cloudflare
    }

    fn get_domain_records(&self, domain_name: &str) -> Result<crate::types::DomainRecordsCommon> {
        let zone_id = self.find_zone_id(domain_name)?;

        let url = format!(
            "{}/zones/{}/dns_records?per_page=5000",
            CLOUDFLARE_API_BASE_URL, zone_id
        );
        debug!("Fetching Cloudflare DNS records from: {}", url);

        let response = self
            .request_client
            .get(&url)
            .bearer_auth(self.token.expose_secret().as_str())
            .send()
            .wrap_err("Failed to query Cloudflare API for DNS records")?;

        let response = handle_error_response(response)?;

        let records_response: CloudflareDnsRecordsResponse = response
            .json()
            .wrap_err("Failed to parse Cloudflare DNS records JSON response")?;

        if !records_response.success {
            bail!(
                "Cloudflare API returned success=false for DNS records of zone '{}'",
                domain_name
            );
        }

        debug!(
            "Found {} DNS records for zone '{}'",
            records_response.result.len(),
            domain_name
        );

        for record in &records_response.result {
            debug!(
                "Retrieved record: id='{}', name='{}', type='{}'",
                record.id, record.name, record.record_type
            );
        }

        let converted_records: Result<Vec<_>, _> = records_response
            .result
            .into_iter()
            .map(|record| CloudflareDnsRecordWithZone::new(record, domain_name).try_into())
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
        // Parse composite ID: "{dns_record_id}/{ttl}/{proxied}"
        let parts: Vec<&str> = domain_record_id.split('/').collect();
        let (dns_record_id, ttl_str, proxied_str) = match parts.as_slice() {
            [id, ttl, proxied] => (*id, *ttl, *proxied),
            _ => {
                bail!(
                    "Invalid Cloudflare record ID format: '{}'. Expected '{{id}}/{{ttl}}/{{proxied}}'",
                    domain_record_id
                );
            }
        };

        let ttl: u32 = ttl_str
            .parse()
            .wrap_err(format!("Failed to parse TTL from record ID: '{}'", ttl_str))?;
        let proxied: bool = proxied_str.parse().wrap_err(format!(
            "Failed to parse proxied flag from record ID: '{}'",
            proxied_str
        ))?;

        let zone_id = self.find_zone_id(&record_to_update.domain_name)?;
        let fqdn = record_to_update.fqdn();

        let url = format!(
            "{}/zones/{}/dns_records/{}",
            CLOUDFLARE_API_BASE_URL, zone_id, dns_record_id
        );
        debug!("Updating Cloudflare DNS record at: {}", url);

        let payload = CloudflareUpdateRecordRequest {
            record_type: record_to_update.record_type.clone(),
            name: fqdn.clone(),
            content: format_record_value(&new_ip.to_string(), &record_to_update.record_type),
            ttl,
            proxied,
        };

        let response = self
            .request_client
            .put(&url)
            .bearer_auth(self.token.expose_secret().as_str())
            .json(&payload)
            .send()
            .wrap_err(format!(
                "Failed to update Cloudflare DNS record for: {}",
                fqdn
            ))?;

        handle_error_response(response)?;

        info!("Successfully updated public IP for: {}", fqdn);
        Ok(())
    }
}

impl Drop for CloudflareApi {
    fn drop(&mut self) {
        tracing::trace!("CloudflareApi object destroyed");
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
                "sample_cloudflare_dns_records_response.json",
            ]
            .iter()
            .collect::<std::path::PathBuf>();
            std::fs::read_to_string(path).expect("Mock domain records not found")
        }

        fn parse_domain_records(s: &str) -> Result<CloudflareDnsRecordsResponse> {
            let records: CloudflareDnsRecordsResponse =
                serde_json::from_str(s).wrap_err("Failed to parse domain records JSON data")?;
            Ok(records)
        }
    }

    impl DomainRecordApi for MockApi {
        fn provider_name(&self) -> &'static str {
            "Mock Cloudflare"
        }

        fn provider_type(&self) -> ProviderType {
            ProviderType::Cloudflare
        }

        fn get_domain_records(
            &self,
            _domain_name: &str,
        ) -> Result<crate::types::DomainRecordsCommon> {
            let s = Self::get_mock_domain_records_response();
            let records_response = Self::parse_domain_records(&s)?;

            let converted_records: Result<Vec<_>, _> = records_response
                .result
                .into_iter()
                .map(|record| CloudflareDnsRecordWithZone::new(record, &self.zone_name).try_into())
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
provider = "cloudflare"
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
            // MockIpFetcher returns 85.212.89.12, fixture contains 1.2.3.4 — they differ
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

            // Verify name normalization: fixture has "home.site.com", expect "home"
            let domain_record = get_record_to_update(&records, &record_to_update).unwrap();
            assert_eq!(domain_record.name, "home");

            // Verify composite ID encoding from fixture (id="rec789id000", ttl=3600, proxied=false)
            assert_eq!(domain_record.id, "rec789id000/3600/false");

            let (ip_addr, _ip_kind) = public_ips.to_ip_addr_from_any();
            let should_update = should_update_domain_ip(&ip_addr, domain_record);
            // IPs differ (85.212.89.12 vs 1.2.3.4), so should update
            assert!(should_update);

            let result = updater.update_domain_ip(&domain_record.id, &record_to_update, &ip_addr);
            assert!(result.is_err());

            Ok(())
        });
    }
}
