use color_eyre::eyre::{bail, eyre, Error, Result, WrapErr};
use reqwest::blocking::Client;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use tracing::{debug, info};

use crate::config::provider_config::{ProviderType, SecretProviderToken};
use crate::domain_record_api::{format_record_value, strip_record_value, DomainRecordApi};
use crate::types::DomainRecordToUpdate;

const DESEC_API_BASE_URL: &str = "https://desec.io/api/v1";

#[derive(Deserialize, Debug, Clone)]
struct DesecRRSet {
    subname: String,
    #[serde(rename = "type")]
    record_type: String,
    records: Vec<String>,
    #[allow(dead_code)]
    ttl: u32,
}

#[derive(Serialize, Debug)]
struct DesecPatchRRSetRequest {
    records: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct DesecErrorResponse {
    detail: Option<String>,
}

/// Handle deSEC API error responses.
fn handle_error_response(
    response: reqwest::blocking::Response,
) -> Result<reqwest::blocking::Response> {
    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .unwrap_or_else(|_| "Unable to read error response".to_string());

        if let Ok(error_response) = serde_json::from_str::<DesecErrorResponse>(&error_text) {
            if let Some(detail) = error_response.detail {
                bail!("deSEC API error ({}): {}", status, detail);
            }
        }

        bail!("HTTP {} error: {}", status, error_text);
    }

    Ok(response)
}

impl TryFrom<DesecRRSet> for crate::types::DomainRecordCommon {
    type Error = Error;

    fn try_from(rrset: DesecRRSet) -> Result<Self, Self::Error> {
        let ip_value = strip_record_value(rrset.records.first().ok_or_else(|| {
            eyre!(
                "RRSet '{}' (type: {}) has no records",
                rrset.subname,
                rrset.record_type
            )
        })?)
        .to_string();

        // Normalize subname: deSEC uses "" for apex, we use "@"
        let hostname_part = if rrset.subname.is_empty() {
            "@".to_string()
        } else {
            rrset.subname.clone()
        };

        // Composite ID: "{subname}/{type}" — same format as Hetzner
        let composite_id = format!("{}/{}", hostname_part, rrset.record_type);

        Ok(Self {
            id: composite_id,
            record_type: rrset.record_type,
            name: hostname_part,
            ip_value,
        })
    }
}

pub struct DesecApi {
    request_client: Client,
    token: SecretProviderToken,
}

impl DesecApi {
    pub fn new(token: SecretProviderToken) -> Self {
        Self {
            request_client: Client::new(),
            token,
        }
    }
}

impl DomainRecordApi for DesecApi {
    fn provider_name(&self) -> &'static str {
        "deSEC"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Desec
    }

    fn get_domain_records(&self, domain_name: &str) -> Result<crate::types::DomainRecordsCommon> {
        let url = format!("{}/domains/{}/rrsets/", DESEC_API_BASE_URL, domain_name);
        debug!("Fetching deSEC RRsets from: {}", url);

        let response = self
            .request_client
            .get(&url)
            .header(
                "Authorization",
                format!("Token {}", self.token.expose_secret().as_str()),
            )
            .send()
            .wrap_err("Failed to query deSEC API for RRsets")?;

        let response = handle_error_response(response)?;

        let rrsets: Vec<DesecRRSet> = response
            .json()
            .wrap_err("Failed to parse deSEC RRsets JSON response")?;

        debug!("Found {} RRsets for domain '{}'", rrsets.len(), domain_name);

        for rrset in &rrsets {
            debug!(
                "Retrieved RRset: subname='{}', type='{}', records={:?}",
                rrset.subname, rrset.record_type, rrset.records
            );
        }

        // Filter out RRsets with emty records and convert
        let converted_records: Result<Vec<_>, _> = rrsets
            .into_iter()
            .filter(|rrset| !rrset.records.is_empty())
            .map(|rrset| rrset.try_into())
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
        let parts: Vec<&str> = domain_record_id.split('/').collect();
        let (subname, rr_type) = match parts.as_slice() {
            [name, record_type] => (*name, *record_type),
            _ => {
                bail!(
                    "Invalid deSEC record ID format: '{}'. Expected '{{subname}}/{{type}}'",
                    domain_record_id
                );
            }
        };

        let fqdn = record_to_update.fqdn();

        // deSEC accepts "@" as a literal path segment for apex records
        let url = format!(
            "{}/domains/{}/rrsets/{}/{}/",
            DESEC_API_BASE_URL, record_to_update.domain_name, subname, rr_type
        );
        debug!("Updating deSEC RRset at: {}", url);

        let payload = DesecPatchRRSetRequest {
            records: vec![format_record_value(
                &new_ip.to_string(),
                &record_to_update.record_type,
            )],
        };

        let response = self
            .request_client
            .patch(&url)
            .header(
                "Authorization",
                format!("Token {}", self.token.expose_secret().as_str()),
            )
            .json(&payload)
            .send()
            .wrap_err(format!("Failed to update deSEC RRset for: {}", fqdn))?;

        handle_error_response(response)?;

        info!("Successfully updated public IP for: {}", fqdn);
        Ok(())
    }
}

impl Drop for DesecApi {
    fn drop(&mut self) {
        tracing::trace!("DesecApi object destroyed");
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
    }

    impl MockApi {
        fn new() -> Self {
            Self {
                return_success: false,
            }
        }

        fn get_mock_rrsets_response() -> String {
            let path = [
                env!("CARGO_MANIFEST_DIR"),
                "tests/data/",
                "sample_desec_rrsets_response.json",
            ]
            .iter()
            .collect::<std::path::PathBuf>();
            std::fs::read_to_string(path).expect("Mock RRsets not found")
        }

        fn parse_rrsets(s: &str) -> Result<Vec<DesecRRSet>> {
            let rrsets: Vec<DesecRRSet> =
                serde_json::from_str(s).wrap_err("Failed to parse RRsets JSON data")?;
            Ok(rrsets)
        }
    }

    impl DomainRecordApi for MockApi {
        fn provider_name(&self) -> &'static str {
            "Mock deSEC"
        }

        fn provider_type(&self) -> ProviderType {
            ProviderType::Desec
        }

        fn get_domain_records(
            &self,
            _domain_name: &str,
        ) -> Result<crate::types::DomainRecordsCommon> {
            let s = Self::get_mock_rrsets_response();
            let rrsets = Self::parse_rrsets(&s)?;

            let converted_records: Result<Vec<_>, _> = rrsets
                .into_iter()
                .filter(|rrset| !rrset.records.is_empty())
                .map(|rrset| rrset.try_into())
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
provider = "desec"
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
            let updater = MockApi::new();
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

            // Verify name is preserved (subname "home" stays "home")
            let domain_record = get_record_to_update(&records, &record_to_update).unwrap();
            assert_eq!(domain_record.name, "home");

            // Verify composite ID: "{subname}/{type}"
            assert_eq!(domain_record.id, "home/A");

            let (ip_addr, _ip_kind) = public_ips.to_ip_addr_from_any();
            let should_update = should_update_domain_ip(&ip_addr, domain_record);
            // IPs differ (85.212.89.12 vs 1.2.3.4), so should update
            assert!(should_update);

            let result = updater.update_domain_ip(&domain_record.id, &record_to_update, &ip_addr);
            // Mock always returns Err
            assert!(result.is_err());

            Ok(())
        });
    }
}
