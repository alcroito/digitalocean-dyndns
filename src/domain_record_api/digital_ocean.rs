use anyhow::{bail, Context, Result};
use log::info;
use reqwest::blocking::Client;
use secrecy::ExposeSecret;
use std::net::IpAddr;

use crate::domain_record_api::DomainRecordApi;
use crate::token::SecretDigitalOceanToken;
use crate::types::{api, DomainRecordToUpdate};

const DIGITAL_OCEAN_API_HOST_NAME: &str = "https://api.digitalocean.com";

pub struct DigitalOceanApi {
    request_client: Client,
    digital_ocean_token: SecretDigitalOceanToken,
}

impl DigitalOceanApi {
    pub fn new(digital_ocean_token: SecretDigitalOceanToken) -> Self {
        Self {
            request_client: Client::new(),
            digital_ocean_token,
        }
    }
}

impl DomainRecordApi for DigitalOceanApi {
    fn get_domain_records(&self, domain_name: &str) -> Result<api::DomainRecords> {
        let endpoint = format!("/v2/domains/{}/records", domain_name);
        let request_url = format!("{}{}", DIGITAL_OCEAN_API_HOST_NAME, endpoint);
        let access_token = &self.digital_ocean_token;
        let response = self
            .request_client
            .get(&request_url)
            .bearer_auth(access_token.expose_secret().as_str())
            .send()
            .context("Failed to query DO for domain records")?;

        let records: api::DomainRecords = response
            .json()
            .context("Failed to parse domain records JSON data")?;
        Ok(records)
    }

    // Extract domain and hostname part into separate struct.
    fn update_domain_ip(
        &self,
        domain_record_id: u64,
        record_to_update: &DomainRecordToUpdate,
        new_ip: &IpAddr,
    ) -> Result<()> {
        let fqdn = record_to_update.fqdn();
        let endpoint = format!(
            "/v2/domains/{}/records/{}",
            record_to_update.domain_name, domain_record_id
        );
        let request_url = format!("{}{}", DIGITAL_OCEAN_API_HOST_NAME, endpoint);
        let access_token = &self.digital_ocean_token;
        let client = Client::new();
        let mut body = std::collections::HashMap::new();
        body.insert("data", new_ip.to_string());
        let response = client
            .put(&request_url)
            .bearer_auth(access_token.expose_secret().as_str())
            .json(&body)
            .send()
            .context(format!("Failed to update domain record for: {}", fqdn))?;

        let record: api::UpdateDomainRecordResponse = response
            .json()
            .context("Failed to parse domain record response JSON data")?;
        let response_ip = record
            .domain_record
            .data
            .parse::<IpAddr>()
            .expect("Failed to parse IP from response");
        if &response_ip != new_ip {
            bail!(format!("Failed to update IP for: {}", fqdn))
        } else {
            info!("Successfully updated public IP for: {}", fqdn);
        }
        Ok(())
    }
}

impl Drop for DigitalOceanApi {
    fn drop(&mut self) {
        info!("Shutting down updater")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ip_fetcher::tests::MockIpFetcher;
    use crate::ip_fetcher::PublicIpFetcher;
    use anyhow::anyhow;
    use std::net::Ipv4Addr;
    struct MockApi {
        return_success: bool,
    }

    #[allow(unused)]
    impl MockApi {
        fn new() -> Self {
            Self {
                return_success: false,
            }
        }

        fn get_mock_public_ip_address() -> IpAddr {
            IpAddr::V4(Ipv4Addr::new(85, 212, 89, 12))
        }

        fn get_mock_domain_records_response() -> String {
            let path = format!("tests/data/{}", "sample_list_domain_records_response.json");
            std::fs::read_to_string(path).expect("Mock domain records not found")
        }

        fn parse_domain_records(s: &str) -> Result<api::DomainRecords> {
            let records: api::DomainRecords =
                serde_json::from_str(&s).context("Failed to parse domain records JSON data")?;
            Ok(records)
        }
    }

    impl DomainRecordApi for MockApi {
        fn get_domain_records(&self, _domain_name: &str) -> Result<api::DomainRecords> {
            let s = Self::get_mock_domain_records_response();
            Self::parse_domain_records(&s)
        }

        #[allow(unused_variables)]
        fn update_domain_ip(
            &self,
            domain_record_id: u64,
            record_to_update: &DomainRecordToUpdate,
            new_ip: &IpAddr,
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
        use crate::types::ValueFromStr;
        use crate::updater::{get_record_to_update, should_update_domain_ip};

        let mut config_builder =
            crate::config_builder::Builder::new(None, Err(anyhow!("No config")));
        config_builder
            .set_subdomain_to_update("home".to_owned())
            .set_domain_root("site.com".to_owned())
            .set_digital_ocean_token(ValueFromStr::from_str("123").unwrap())
            .set_log_level(log::LevelFilter::Info)
            .set_update_interval(crate::config::UpdateInterval(
                std::time::Duration::from_secs(5).into(),
            ));
        let config = config_builder.build().unwrap();
        let ip_fetcher = MockIpFetcher::default();
        let public_ip = ip_fetcher.fetch_public_ip().unwrap();
        let updater = MockApi::new();
        let domain_name = &config.domains.domains[0].name;
        let hostname_part = &config.domains.domains[0].records[0].name;
        let record_type = "A";
        let record_to_update =
            DomainRecordToUpdate::new(&domain_name, &hostname_part, &record_type);

        let records = updater.get_domain_records(domain_name).unwrap();
        let domain_record = get_record_to_update(&records, &record_to_update).unwrap();
        let should_update = should_update_domain_ip(&public_ip, domain_record);

        assert_eq!(should_update, true);

        let result = updater.update_domain_ip(domain_record.id, &record_to_update, &public_ip);
        assert!(result.is_err());
    }
}
