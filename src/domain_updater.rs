use anyhow::{anyhow, bail, Context, Result};
use humantime::format_duration;
use log::{error, info, trace, warn};
use reqwest::blocking::Client;
use secrecy::ExposeSecret;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{park_timeout, JoinHandle};
use std::time::Instant;

use crate::config::Config;
use crate::ip_fetcher::{DnsIpFetcher, PublicIpFetcher};
use crate::types::{DomainFilter, DomainRecord, DomainRecords, UpdateDomainRecordResponse};

const DIGITAL_OCEAN_API_HOST_NAME: &str = "https://api.digitalocean.com";

trait DomainRecordUpdater {
    fn get_domain_records(&self) -> Result<DomainRecords>;
    fn update_domain_ip(
        &self,
        domain_record_id: u64,
        domain_root: &str,
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
        let domain_record = get_record_to_update(&records, &self.config.hostname_part)?;
        if should_update_domain_ip(&public_ip, &domain_record) {
            info!(
                "Existing domain record IP does not match current public IP: '{}'; domain record IP: '{}'. Updating record",
                public_ip, domain_record.data
            );
            let domain_root = &self.config.domain_root;
            if !self.config.dry_run {
                self.update_domain_ip(domain_record.id, domain_root, &public_ip)?;
            } else {
                info!("Skipping updating IP due to dry run")
            }
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

        let domain_filter = DomainFilter::new(&self.config.domain_root, &self.config.hostname_part);
        let fqdn = domain_filter.fqdn();
        let duration_formatted = format_duration(*self.config.update_interval.0);
        info!(
            "Starting updater: domain record '{}' of type '{}' will be updated every {}",
            fqdn,
            domain_filter.record_type(),
            duration_formatted
        );

        loop {
            let attempt_result = self.attempt_update(&ip_fetcher);
            if let Err(e) = attempt_result {
                error!("Domain record update attempt failed: {}", e);
                self.failed_attempts += 1;
            }
            if self.failed_attempts > 10 {
                warn!("Too many failed domain record update attempts. Shutting down updater");
                break;
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

    fn build_query_filter(&self) -> Vec<(&'static str, String)> {
        let domain_filter = DomainFilter::new(&self.config.domain_root, &self.config.hostname_part);
        let record_type = match domain_filter {
            DomainFilter::Root(_) => domain_filter.record_type().to_owned(),
            DomainFilter::Subdomain(_) => domain_filter.record_type().to_owned(),
        };
        vec![("type", record_type), ("name", domain_filter.fqdn())]
    }
}

impl DomainRecordUpdater for DigitalOceanUpdater {
    fn get_domain_records(&self) -> Result<DomainRecords> {
        let domain_root = &self.config.domain_root;
        let endpoint = format!("/v2/domains/{}/records", domain_root);
        let request_url = format!("{}{}", DIGITAL_OCEAN_API_HOST_NAME, endpoint);
        let access_token = &self.config.digital_ocean_token;
        let query_filter = self.build_query_filter();
        let response = self
            .request_client
            .get(&request_url)
            .bearer_auth(access_token.expose_secret().as_str())
            .query(&query_filter)
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
        new_ip: &IpAddr,
    ) -> Result<()> {
        let domain_filter = DomainFilter::new(&self.config.domain_root, &self.config.hostname_part);
        let fqdn = domain_filter.fqdn();
        let endpoint = format!("/v2/domains/{}/records/{}", domain_root, domain_record_id);
        let request_url = format!("{}{}", DIGITAL_OCEAN_API_HOST_NAME, endpoint);
        let access_token = &self.config.digital_ocean_token;
        let client = Client::new();
        let mut body = std::collections::HashMap::new();
        body.insert("data", new_ip.to_string());
        let response = client
            .put(&request_url)
            .bearer_auth(access_token.expose_secret().as_str())
            .json(&body)
            .send()
            .context(format!("Failed to update domain record for: {}", fqdn))?;

        let record: UpdateDomainRecordResponse = response
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

impl Drop for DigitalOceanUpdater {
    fn drop(&mut self) {
        info!("Shutting down updater")
    }
}

fn get_record_to_update<'a>(
    records: &'a DomainRecords,
    hostname_part: &str,
) -> Result<&'a DomainRecord> {
    if records.domain_records.is_empty() {
        bail!("Failed to find hostname to update, retreived domain records are empty");
    }
    records
        .domain_records
        .iter()
        .find(|&record| record.name.eq(hostname_part))
        .ok_or_else(|| anyhow!("Failed to find hostname to update in the retrieved domain records"))
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
        let updater = MockUpdater::new();
        let records = updater.get_domain_records().unwrap();
        let domain_record = get_record_to_update(&records, &config.hostname_part).unwrap();
        let should_update = should_update_domain_ip(&public_ip, domain_record);

        assert_eq!(should_update, true);

        let result = updater.update_domain_ip(domain_record.id, &config.domain_root, &public_ip);
        assert!(result.is_err());
    }
}
