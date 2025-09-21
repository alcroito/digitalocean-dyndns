use color_eyre::eyre::{bail, Error, Result, WrapErr};
use reqwest::blocking::Client;
use secrecy::ExposeSecret;
use serde::Deserialize;
use std::net::IpAddr;
use tracing::{info, trace};

use crate::config::provider_config::{ProviderType, SecretProviderToken};
use crate::domain_record_api::DomainRecordApi;
use crate::types::{DomainRecordCommon, DomainRecordToUpdate, DomainRecordsCommon};

#[derive(Deserialize, Debug)]
pub struct DomainRecordDigitalOcean {
    pub id: u64,
    #[serde(rename = "type")]
    pub record_type: String,
    pub name: String,
    // This contains the API response IP address.
    pub data: String,
}

#[derive(Deserialize, Debug)]
pub struct DomainRecordsDigitalOcean {
    pub domain_records: Vec<DomainRecordDigitalOcean>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateDomainRecordResponse {
    pub domain_record: DomainRecordDigitalOcean,
}

impl TryFrom<DomainRecordDigitalOcean> for DomainRecordCommon {
    type Error = Error;

    fn try_from(record: DomainRecordDigitalOcean) -> Result<Self> {
        Ok(Self {
            id: record.id.to_string(),
            record_type: record.record_type,
            name: record.name,
            ip_value: record.data,
        })
    }
}

impl TryFrom<DomainRecordsDigitalOcean> for DomainRecordsCommon {
    type Error = Error;

    fn try_from(records: DomainRecordsDigitalOcean) -> Result<Self> {
        let converted_records: Result<Vec<_>, _> = records
            .domain_records
            .into_iter()
            .map(|record| record.try_into())
            .collect();

        Ok(Self {
            records: converted_records?,
        })
    }
}

const DIGITAL_OCEAN_API_HOST_NAME: &str = "https://api.digitalocean.com";

pub struct DigitalOceanApi {
    request_client: Client,
    token: SecretProviderToken,
}

impl DigitalOceanApi {
    pub fn new(token: SecretProviderToken) -> Self {
        Self {
            request_client: Client::new(),
            token,
        }
    }
}

impl DomainRecordApi for DigitalOceanApi {
    fn provider_name(&self) -> &'static str {
        "DigitalOcean"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::DigitalOcean
    }

    fn get_domain_records(&self, domain_name: &str) -> Result<DomainRecordsCommon> {
        let endpoint = format!("/v2/domains/{domain_name}/records?per_page=200");
        let request_url = format!("{DIGITAL_OCEAN_API_HOST_NAME}{endpoint}");
        let response = self
            .request_client
            .get(request_url)
            .bearer_auth(self.token.expose_secret().as_str())
            .send()
            .wrap_err("Failed to query DO for domain records")?;
        let response_text = response
            .text()
            .wrap_err("Failed to retrieve domain records response text")?;
        let records: DomainRecordsDigitalOcean =
            serde_json::from_str(&response_text).wrap_err(format!(
                "Failed to parse domain records JSON data. Response text: {}",
                &response_text
            ))?;
        records.try_into()
    }

    // Extract domain and hostname part into separate struct.
    fn update_domain_ip(
        &self,
        domain_record_id: &str,
        record_to_update: &DomainRecordToUpdate,
        new_ip: &IpAddr,
    ) -> Result<()> {
        let fqdn = record_to_update.fqdn();
        let endpoint = format!(
            "/v2/domains/{}/records/{}",
            record_to_update.domain_name, domain_record_id
        );
        let request_url = format!("{DIGITAL_OCEAN_API_HOST_NAME}{endpoint}");
        let client = Client::new();
        let mut body = std::collections::HashMap::new();
        body.insert("data", new_ip.to_string());
        let response = client
            .put(request_url)
            .bearer_auth(self.token.expose_secret().as_str())
            .json(&body)
            .send()
            .wrap_err(format!("Failed to update domain record for: {fqdn}"))?;

        let record: UpdateDomainRecordResponse = response
            .json()
            .wrap_err("Failed to parse domain record response JSON data")?;
        let response_ip = record
            .domain_record
            .data
            .parse::<IpAddr>()
            .expect("Failed to parse IP from response");
        if &response_ip != new_ip {
            bail!(format!("Failed to update IP for: {fqdn}"))
        } else {
            info!("Successfully updated public IP for: {}", fqdn);
        }
        Ok(())
    }
}

impl Drop for DigitalOceanApi {
    fn drop(&mut self) {
        trace!("DigitalOceanApi object destroyed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ip_fetcher::tests::MockIpFetcher;
    use crate::ip_fetcher::PublicIpFetcher;
    struct MockApi {
        return_success: bool,
    }

    impl MockApi {
        fn new() -> Self {
            Self {
                return_success: false,
            }
        }

        fn get_mock_domain_records_response() -> String {
            let path = [
                env!("CARGO_MANIFEST_DIR"),
                "tests/data/",
                "sample_list_domain_records_response.json",
            ]
            .iter()
            .collect::<std::path::PathBuf>();
            std::fs::read_to_string(path).expect("Mock domain records not found")
        }

        fn parse_domain_records(s: &str) -> Result<DomainRecordsDigitalOcean> {
            let records: DomainRecordsDigitalOcean =
                serde_json::from_str(s).wrap_err("Failed to parse domain records JSON data")?;
            Ok(records)
        }
    }

    impl DomainRecordApi for MockApi {
        fn provider_name(&self) -> &'static str {
            "Mock"
        }

        fn provider_type(&self) -> ProviderType {
            ProviderType::DigitalOcean
        }

        fn get_domain_records(&self, _domain_name: &str) -> Result<DomainRecordsCommon> {
            let s = Self::get_mock_domain_records_response();
            Self::parse_domain_records(&s).and_then(|records| records.try_into())
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
digital_ocean_token = "123"
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
            let updater = MockApi::new();
            let domain_name = &config.domains.domains.first().expect("no domain").name;
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
