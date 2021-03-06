use anyhow::{anyhow, bail, Result};
use humantime::format_duration;
use log::{error, info, trace, warn};
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{park_timeout, JoinHandle};
use std::time::Instant;

use crate::config;
use crate::config::Config;
use crate::domain_record_api::DomainRecordApi;
use crate::ip_fetcher::{DnsIpFetcher, PublicIpFetcher};
use crate::types::{api, DomainRecordToUpdate};

pub struct Updater {
    config: Config,
    api: Box<dyn DomainRecordApi + Send>,
    failed_attempts: u64,
    should_exit: Arc<AtomicBool>,
}

impl Updater {
    pub fn new(config: Config, api: Box<dyn DomainRecordApi + Send>) -> Self {
        Self {
            config,
            api,
            failed_attempts: 0,
            should_exit: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn exit_flag(&self) -> Arc<AtomicBool> {
        self.should_exit.clone()
    }

    fn attempt_update_for_record(
        &self,
        public_ip: &IpAddr,
        record_to_update: &DomainRecordToUpdate,
        domain_record_cache: &mut api::DomainRecordCache,
    ) -> Result<()> {
        info!(
            "Attempting to update domain record '{}'",
            record_to_update.fqdn()
        );
        let records = match domain_record_cache.entry(record_to_update.domain_name.to_string()) {
            std::collections::hash_map::Entry::<_, _>::Vacant(o) => {
                trace!("Querying records for '{}'", record_to_update.domain_name);
                let records = self.api.get_domain_records(record_to_update.domain_name)?;
                o.insert(records)
            }
            std::collections::hash_map::Entry::<_, _>::Occupied(o) => {
                trace!(
                    "Reusing cached records for '{}'",
                    record_to_update.domain_name
                );
                o.into_mut()
            }
        };
        let api_domain_record = get_record_to_update(&records, &record_to_update)?;
        if should_update_domain_ip(&public_ip, &api_domain_record) {
            info!(
                "Existing domain record IP does not match current public IP: '{}'; domain record IP: '{}'. Updating record",
                public_ip, api_domain_record.data
            );
            if !self.config.dry_run {
                self.api
                    .update_domain_ip(api_domain_record.id, &record_to_update, &public_ip)?;
            } else {
                info!("Skipping updating IP due to dry run")
            }
        } else {
            info!("Correct IP already set, nothing to do");
        }
        Ok(())
    }

    fn attempt_update(
        &self,
        ip_fetcher: &DnsIpFetcher,
        records_to_update: &[DomainRecordToUpdate],
    ) -> Result<()> {
        let public_ip = ip_fetcher.fetch_public_ip()?;

        let mut first_error = None;
        let mut domain_record_cache = api::DomainRecordCache::new();

        for record_to_update in records_to_update {
            if let Err(e) = self.attempt_update_for_record(
                &public_ip,
                &record_to_update,
                &mut domain_record_cache,
            ) {
                error!("{}", e);
                first_error = Some(e);
            }
        }
        if let Some(e) = first_error {
            return Err(e);
        }
        Ok(())
    }

    pub fn start_update_loop_detached(mut self) -> JoinHandle<Result<()>> {
        std::thread::spawn(move || self.start_update_loop())
    }

    fn build_starting_updater_mesage(
        interval: &config::UpdateInterval,
        records_to_update: &[DomainRecordToUpdate],
    ) -> String {
        let duration_formatted = format_duration(*interval.0);
        let m = format!(
            "Starting updater with update interval: {}. The following domains will be updated:",
            duration_formatted
        );
        let mut record_m = vec![];
        for record in records_to_update {
            let fqdn = record.fqdn();
            record_m.push(format!(
                "    domain record '{}' of type '{}'",
                fqdn, record.record_type
            ));
        }
        format!("{}\n{}", m, record_m.join("\n"))
    }

    fn build_records_to_update(config: &Config) -> Vec<DomainRecordToUpdate> {
        config
            .domains
            .domains
            .iter()
            .flat_map(|domain| {
                let domain_name = domain.name.as_str();
                domain.records.iter().map(move |record| {
                    DomainRecordToUpdate::new(
                        domain_name,
                        record.name.as_str(),
                        record.record_type.as_str(),
                    )
                })
            })
            .collect::<Vec<_>>()
    }

    pub fn start_update_loop(&mut self) -> Result<()> {
        let ip_fetcher = DnsIpFetcher::default();
        let records_to_update = Updater::build_records_to_update(&self.config);

        let starting_message = Updater::build_starting_updater_mesage(
            &self.config.update_interval,
            &records_to_update,
        );
        info!("{}", starting_message);

        loop {
            let attempt_result = self.attempt_update(&ip_fetcher, &records_to_update);
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
}

pub fn get_record_to_update<'a>(
    records: &'a api::DomainRecords,
    record_to_update: &DomainRecordToUpdate,
) -> Result<&'a api::DomainRecord> {
    if records.domain_records.is_empty() {
        bail!(format!(
            "Failed to find domain '{}', retrieved domain records are empty",
            record_to_update.fqdn()
        ));
    }
    records
        .domain_records
        .iter()
        .find(|&record| {
            record.name.eq(record_to_update.hostname_part)
                && record.record_type.eq(record_to_update.record_type)
        })
        .ok_or_else(|| {
            anyhow!(format!(
                "Domain '{}' not found in the retrieved domain records",
                record_to_update.fqdn()
            ))
        })
}

pub fn should_update_domain_ip(public_ip: &IpAddr, domain_record: &api::DomainRecord) -> bool {
    let ip = &domain_record.data;
    let ip = ip
        .parse::<IpAddr>()
        .expect("Failed parsing string to IP address in domain record");
    public_ip != &ip
}
