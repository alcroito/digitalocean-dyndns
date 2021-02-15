use anyhow::{anyhow, bail, Context, Result};
use humantime::format_duration;
use log::{error, info, trace, warn};
use reqwest::blocking::Client;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{park_timeout, JoinHandle};
use std::time::Instant;

use crate::config::Config;
use crate::ip_fetcher::{DnsIpFetcher, PublicIpFetcher};
use crate::types::{DomainRecord, DomainRecords, UpdateDomainRecordResponse};

const DIGITAL_OCEAN_API_HOST_NAME: &str = "https://api.digitalocean.com";

trait DomainRecordUpdater {
    fn get_domain_records(&self) -> Result<DomainRecords>;
    fn update_domain_ip(
        &self,
        domain_record_id: u64,
        domain_root: &str,
        subdomain: &str,
        new_ip: &IpAddr,
    ) -> Result<()>;
}

pub struct DigitalOceanUpdater {
    config: Config,
    request_client: Client,
    failed_attempts: u64,
    should_exit: Arc<AtomicBool>,
}

impl DigitalOceanUpdater {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            request_client: Client::new(),
            failed_attempts: 0,
            should_exit: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn exit_flag(&self) -> Arc<AtomicBool> {
        self.should_exit.clone()
    }

    fn attempt_update(&self, ip_fetcher: &DnsIpFetcher) -> Result<()> {
        let public_ip = ip_fetcher.fetch_public_ip()?;

        info!("Attempting domain record update");
        let records = self.get_domain_records()?;
        let domain_record =
            get_subdomain_record_to_update(&records, &self.config.subdomain_to_update)?;
        if should_update_domain_ip(&public_ip, &domain_record) {
            info!(
                "Existing domain record IP does not match current public IP: '{}'; domain record IP: '{}'. Updating record",
                public_ip, domain_record.data
            );
            let domain_root = &self.config.domain_root;
            let subdomain = &self.config.subdomain_to_update;
            self.update_domain_ip(domain_record.id, domain_root, subdomain, &public_ip)?;
        } else {
            info!("Correct IP already set, nothing to do");
        }
        Ok(())
    }

    pub fn start_update_loop_detached(mut self) -> JoinHandle<Result<()>> {
        std::thread::spawn(move || self.start_update_loop())
    }

    pub fn start_update_loop(&mut self) -> Result<()> {
        let ip_fetcher = DnsIpFetcher::default();

        let fqdn = build_subdomain_fqdn(&self.config.domain_root, &self.config.subdomain_to_update);
        let duration_formatted = format_duration(*self.config.update_interval.0);
        info!(
            "Starting updater: domain record '{}' will be updated every {}",
            fqdn, duration_formatted
        );

        loop {
            if !self.config.dry_run {
                let attempt_result = self.attempt_update(&ip_fetcher);
                if let Err(e) = attempt_result {
                    error!("Domain record update attempt failed: {}", e);
                    self.failed_attempts += 1;
                }
                if self.failed_attempts > 10 {
                    warn!("Too many failed domain record update attempts. Shutting down updater");
                    break;
                }
            }

            let duration_formatted = format_duration(*self.config.update_interval.0);
            trace!("Sleeping for {}", duration_formatted);

            // Exit if interrupted.
            if self.was_interrupted_while_sleeping() {
                trace!("Process was requested to exit either by system or user");
                return Ok(());
            }
        }
        Ok(())
    }

    fn was_interrupted_while_sleeping(&self) -> bool {
        let beginning_park = Instant::now();
        let timeout = self.config.update_interval.0;
        let mut sleep_time_left = timeout;
        loop {
            park_timeout(*sleep_time_left);
            let elapsed = beginning_park.elapsed();
            trace!("Interrupted, elapsed {:?}", elapsed);
            if self.should_exit() {
                return true;
            }
            if elapsed >= *timeout {
                break;
            }
            trace!("restarting park_timeout after {:?}", elapsed);
            sleep_time_left = (*timeout - elapsed).into();
        }
        false
    }

    fn should_exit(&self) -> bool {
        self.should_exit.load(Ordering::SeqCst)
    }
}

impl DomainRecordUpdater for DigitalOceanUpdater {
    fn get_domain_records(&self) -> Result<DomainRecords> {
        let domain_root = &self.config.domain_root;
        let subdomain = &self.config.subdomain_to_update;
        let endpoint = format!("/v2/domains/{}/records", domain_root);
        let request_url = format!("{}{}", DIGITAL_OCEAN_API_HOST_NAME, endpoint);
        let access_token = &self.config.digital_ocean_token;
        let subdomain_filter = build_subdomain_fqdn(&domain_root, &subdomain);
        let response = self
            .request_client
            .get(&request_url)
            .bearer_auth(access_token)
            .query(&[("name", &subdomain_filter)])
            .send()
            .context("Failed to query DO for domain records")?;

        let records: DomainRecords = response
            .json()
            .context("Failed to parse domain records JSON data")?;
        Ok(records)
    }

    fn update_domain_ip(
        &self,
        domain_record_id: u64,
        domain_root: &str,
        subdomain: &str,
        new_ip: &IpAddr,
    ) -> Result<()> {
        let subdomain = build_subdomain_fqdn(&domain_root, &subdomain);
        let endpoint = format!("/v2/domains/{}/records/{}", domain_root, domain_record_id);
        let request_url = format!("{}{}", DIGITAL_OCEAN_API_HOST_NAME, endpoint);
        let access_token = &self.config.digital_ocean_token;
        let client = Client::new();
        let mut body = std::collections::HashMap::new();
        body.insert("data", new_ip.to_string());
        let response = client
            .put(&request_url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .context(format!(
                "Failed to update domain record for subdomain: {}",
                subdomain
            ))?;

        let record: UpdateDomainRecordResponse = response
            .json()
            .context("Failed to parse domain record response JSON data")?;
        let response_ip = record
            .domain_record
            .data
            .parse::<IpAddr>()
            .expect("Failed to parse IP from response");
        if &response_ip != new_ip {
            bail!(format!("Failed to update IP for subdomain: {}", subdomain))
        } else {
            info!(
                "Successfully updated public IP for subdomain: {}",
                subdomain
            );
        }
        Ok(())
    }
}

impl Drop for DigitalOceanUpdater {
    fn drop(&mut self) {
        info!("Shutting down updater")
    }
}

fn build_subdomain_fqdn(domain_root: &str, subdomain: &str) -> String {
    format!("{}.{}", subdomain, domain_root)
}

fn get_subdomain_record_to_update<'a>(
    records: &'a DomainRecords,
    subdomain: &str,
) -> Result<&'a DomainRecord> {
    if records.domain_records.is_empty() {
        bail!("Failed to find subdomain update, domain records are empty");
    }
    records
        .domain_records
        .iter()
        .find(|&record| record.name.eq(subdomain))
        .ok_or_else(|| anyhow!("Failed to find subdomain to update"))
}

fn should_update_domain_ip(public_ip: &IpAddr, domain_record: &DomainRecord) -> bool {
    let ip = &domain_record.data;
    let ip = ip
        .parse::<IpAddr>()
        .expect("Failed parsing string to IP address in domain record");
    public_ip != &ip
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ip_fetcher::tests::MockIpFetcher;
    use std::net::Ipv4Addr;

    struct MockUpdater {
        return_success: bool,
    }

    #[allow(unused)]
    impl MockUpdater {
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

        fn parse_domain_records(s: &str) -> Result<DomainRecords> {
            let records: DomainRecords =
                serde_json::from_str(&s).context("Failed to parse domain records JSON data")?;
            Ok(records)
        }
    }

    impl DomainRecordUpdater for MockUpdater {
        fn get_domain_records(&self) -> Result<DomainRecords> {
            let s = Self::get_mock_domain_records_response();
            Self::parse_domain_records(&s)
        }

        #[allow(unused_variables)]
        fn update_domain_ip(
            &self,
            domain_record_id: u64,
            domain_root: &str,
            subdomain: &str,
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
        let mut config_builder =
            crate::config_builder::Builder::new(None, Err(anyhow!("No config")));
        config_builder
            .set_subdomain_to_update("home".to_owned())
            .set_domain_root("site.com".to_owned())
            .set_digital_ocean_token("123".to_owned())
            .set_log_level(log::LevelFilter::Info)
            .set_update_interval(crate::config::UpdateInterval(
                std::time::Duration::from_secs(5).into(),
            ));
        let config = config_builder.build().unwrap();
        let ip_fetcher = MockIpFetcher::default();
        let public_ip = ip_fetcher.fetch_public_ip().unwrap();
        let updater = MockUpdater::new();
        let records = updater.get_domain_records().unwrap();
        let domain_record =
            get_subdomain_record_to_update(&records, &config.subdomain_to_update).unwrap();
        let should_update = should_update_domain_ip(&public_ip, domain_record);

        assert_eq!(should_update, true);

        let result = updater.update_domain_ip(
            domain_record.id,
            &config.domain_root,
            &config.subdomain_to_update,
            &public_ip,
        );
        assert!(result.is_err());
    }
}
