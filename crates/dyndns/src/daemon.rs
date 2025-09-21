use crate::config::app_config::GeneralOptions;
use crate::config::provider_config::ProviderType;
use crate::domain_record_api::digital_ocean_api::DigitalOceanApi;
use crate::domain_record_api::{create_provider, DomainRecordApi};
use crate::global_state::GlobalState;
use crate::logger::setup_logger;
use crate::signal_handlers::{setup_forceful_term_signal_handling, AppTerminationHandler};
use crate::updater::Updater;
use color_eyre::eyre::{bail, Result};
use std::collections::HashSet;
use tracing::warn;

#[cfg(feature = "web")]
use crate::web::server::start_web_server_and_wait;

pub fn start_daemon(global_state: GlobalState) -> Result<()> {
    setup_logger(&global_state.config.general_options.log_level)?;
    setup_forceful_term_signal_handling()?;

    let dns_providers = create_dns_providers(&global_state.config.general_options)?;

    let term_handler = AppTerminationHandler::new()?;
    term_handler.setup_exit_panic_hook();

    #[cfg(feature = "web")]
    start_web_server_and_wait(term_handler.clone(), &global_state.config);

    let updater = Updater::new(global_state, dns_providers, term_handler.clone());
    let updater_thread_handle = updater.start_update_loop_detached();
    term_handler.set_updater_thread(updater_thread_handle);
    term_handler.handle_term_signals_gracefully()?;
    Ok(())
}

fn create_dns_providers(
    general_options: &GeneralOptions,
) -> Result<Vec<Box<dyn DomainRecordApi + Send + 'static>>> {
    let mut dns_providers: Vec<Box<dyn DomainRecordApi + Send>> = vec![];
    let mut added_provider_types: HashSet<ProviderType> = HashSet::new();
    if !general_options.providers_config.providers.is_empty() {
        for provider_config in &general_options.providers_config.providers {
            let provider = create_provider(provider_config)?;
            added_provider_types.insert(provider_config.provider);
            dns_providers.push(provider);
        }
    }
    if let Some(token) = general_options.digital_ocean_token.clone() {
        if added_provider_types.contains(&ProviderType::DigitalOcean) {
            warn!("DigitalOcean provider already configured via [[providers]] - skipping digital_ocean_token field.");
        } else {
            let provider = Box::new(DigitalOceanApi::new(token));
            dns_providers.push(provider);
        }
    }
    if dns_providers.is_empty() {
        bail!("At least one DNS provider must be configured. Specify one via the [[providers]] configuration or on the command line");
    }
    Ok(dns_providers)
}
