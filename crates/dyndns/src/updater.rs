use color_eyre::eyre::{bail, eyre, Result};
use humantime::format_duration;
use std::net::IpAddr;
use std::thread::{park_timeout, JoinHandle};
use std::time::Instant;
use tracing::{error, info, trace, warn};

use crate::config::app_config;
use crate::config::app_config::AppConfig;
use crate::domain_record_api::DomainRecordApi;
use crate::global_state::GlobalState;
use crate::ip_fetcher::{DnsIpFetcher, PublicIpFetcher};
use crate::signal_handlers::AppTerminationHandler;
use crate::stats_handler::{StatsHandler, StatsHandlerFactory};
use crate::types::{api, DomainRecordToUpdate, IpAddrKind, IpAddrV4AndV6};
pub struct Updater {
    global_state: GlobalState,
    api: Box<dyn DomainRecordApi + Send>,
    failed_attempts: u64,
    stats_handler: Box<dyn StatsHandler>,
    term_handler: AppTerminationHandler,
}

impl Updater {
    pub fn new(
        global_state: GlobalState,
        api: Box<dyn DomainRecordApi + Send>,
        term_handler: AppTerminationHandler,
    ) -> Self {
        let config = global_state.config.clone();
        Self {
            global_state,
            api,
            failed_attempts: 0,
            stats_handler: StatsHandlerFactory::new_handler(config),
            term_handler,
        }
    }

    fn attempt_update_for_record(
        &mut self,
        current_public_ips: &IpAddrV4AndV6,
        record_to_update: &DomainRecordToUpdate,
        domain_record_cache: &mut api::DomainRecordCache,
    ) -> Result<Option<IpAddrKind>> {
        info!(
            "Attempting to update domain record '{}'",
            record_to_update.fqdn()
        );
        let records = match domain_record_cache.entry(record_to_update.domain_name.clone()) {
            std::collections::hash_map::Entry::<_, _>::Vacant(o) => {
                trace!("Querying records for '{}'", record_to_update.domain_name);
                let records = self.api.get_domain_records(&record_to_update.domain_name)?;
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
        let api_domain_record = get_record_to_update(records, record_to_update)?;
        let mut ip_kind_res = None;
        if let Some((curr_ip, curr_ip_kind)) =
            get_single_ip_based_on_record_type(current_public_ips, api_domain_record)
        {
            ip_kind_res = Some(curr_ip_kind);
            if should_update_domain_ip(&curr_ip, api_domain_record) {
                info!(
                    "Old domain record IP does not match current IP\n  current public IP:    '{}'\n  old domain record IP: '{}'.\nUpdating domain record",
                    curr_ip, api_domain_record.data
                );
                if !self.global_state.config.general_options.dry_run {
                    self.api
                        .update_domain_ip(api_domain_record.id, record_to_update, &curr_ip)?;
                } else {
                    info!("Skipping updating IP due to dry run");
                }
            } else {
                info!("Correct IP already set, nothing to do");
            }
        };

        Ok(ip_kind_res)
    }

    fn attempt_update(
        &mut self,
        ip_fetcher: &DnsIpFetcher,
        records_to_update: &[DomainRecordToUpdate],
    ) -> Result<()> {
        let current_public_ips_result = ip_fetcher.fetch_public_ips(
            self.global_state.config.general_options.ipv4,
            self.global_state.config.general_options.ipv6,
        );
        let (maybe_current_public_ips, maybe_fetched_ip_err) = match current_public_ips_result {
            Ok(v) => (Some(v), None),
            Err(e) => (None, Some(e)),
        };
        if let Some(ref e) = maybe_fetched_ip_err {
            error!("Ip fetching failed: {}", e);
        }

        self.stats_handler
            .handle_ip_fetch(maybe_current_public_ips.clone())?;

        let mut first_error = maybe_fetched_ip_err;
        let mut domain_record_cache = api::DomainRecordCache::new();

        for record_to_update in records_to_update {
            let mut is_domain_record_update_successful = false;
            let mut stats_ip_kind = None;
            if let Some(ref current_public_ips) = maybe_current_public_ips {
                match self.attempt_update_for_record(
                    current_public_ips,
                    record_to_update,
                    &mut domain_record_cache,
                ) {
                    Ok(maybe_ip_kind) => {
                        is_domain_record_update_successful = true;
                        stats_ip_kind = maybe_ip_kind;
                    }
                    Err(e) => {
                        error!("{}", e);
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                    }
                }
            }

            self.stats_handler.handle_updater_attempt(
                &record_to_update.fqdn(),
                &record_to_update.record_type,
                is_domain_record_update_successful,
                stats_ip_kind,
            )?;
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
        interval: &app_config::UpdateInterval,
        records_to_update: &[DomainRecordToUpdate],
    ) -> String {
        let duration_formatted = format_duration(interval.0);
        let m = format!(
            "Starting updater with update interval: {duration_formatted}. The following domain records will be updated:",
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

    fn build_records_to_update(config: &AppConfig) -> Vec<DomainRecordToUpdate> {
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
        let records_to_update = Updater::build_records_to_update(&self.global_state.config);

        self.stats_handler.init()?;

        let starting_message = Updater::build_starting_updater_mesage(
            &self.global_state.config.general_options.update_interval,
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

            let duration_formatted =
                format_duration(self.global_state.config.general_options.update_interval.0);
            trace!("Sleeping for {}", duration_formatted);

            // Exit if interrupted.
            if self.was_interrupted_while_sleeping() {
                info!("Updater recieved signal to shut down. Shutting down");
                return Ok(());
            }
        }
        self.term_handler.notify_exit_and_stop_signal_handling();
        Ok(())
    }

    fn was_interrupted_while_sleeping(&self) -> bool {
        if self.should_exit() {
            return true;
        }

        let beginning_park = Instant::now();
        let timeout = self.global_state.config.general_options.update_interval.0;
        let mut sleep_time_left = timeout;
        loop {
            park_timeout(sleep_time_left);
            let elapsed = beginning_park.elapsed();
            trace!("Interrupted, elapsed {:?}", elapsed);
            if self.should_exit() {
                return true;
            }
            if elapsed >= timeout {
                break;
            }
            trace!("restarting park_timeout after {:?}", elapsed);
            sleep_time_left = timeout - elapsed;
        }
        false
    }

    fn should_exit(&self) -> bool {
        self.term_handler.should_exit()
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
            record.name.eq(&record_to_update.hostname_part)
                && record.record_type.eq(&record_to_update.record_type)
        })
        .ok_or_else(|| {
            eyre!(format!(
                "Domain '{}' not found in the retrieved domain records",
                record_to_update.fqdn()
            ))
        })
}

pub fn get_single_ip_based_on_record_type(
    curr_ips: &IpAddrV4AndV6,
    domain_record: &api::DomainRecord,
) -> Option<(IpAddr, IpAddrKind)> {
    // Choose which ip family type to compare based on the domain record type.
    // For generic record types, use any available ip.
    let record_type = domain_record.record_type.as_str();
    match record_type {
        "A" if curr_ips.has_ipv4() => Some((curr_ips.to_ip_addr_from_ipv4(), IpAddrKind::V4)),
        "AAAA" if curr_ips.has_ipv6() => Some((curr_ips.to_ip_addr_from_ipv6(), IpAddrKind::V6)),
        _ => {
            if record_type != "A" && record_type != "AAAA" && curr_ips.has_any() {
                info!("Non-standard domain record type: '{}', will use any available IP address (either ipv4 of ipv6)", record_type);
                Some(curr_ips.to_ip_addr_from_any())
            } else {
                warn!("No valid IP available for record type '{}', will skip updaing this record type.", record_type);
                None
            }
        }
    }
}

pub fn should_update_domain_ip(curr_ip: &IpAddr, domain_record: &api::DomainRecord) -> bool {
    let prev_ip = domain_record.data.as_str();
    let prev_ip = prev_ip
        .parse::<IpAddr>()
        .unwrap_or_else(|_|
            panic!("Failed parsing string '{}' to IP address in domain record, make sure your domain record has an initial valid ip", prev_ip));

    curr_ip != &prev_ip
}
