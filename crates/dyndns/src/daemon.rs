use crate::config::app_config::AppConfig;
use crate::domain_record_api::digital_ocean::DigitalOceanApi;
use crate::logger::setup_logger;
use crate::signal_handlers::{setup_forceful_term_signal_handling, AppTerminationHandler};

use crate::updater::Updater;
use color_eyre::eyre::Result;

pub fn start_daemon(mut config: AppConfig) -> Result<()> {
    setup_logger(&config.log_level)?;
    setup_forceful_term_signal_handling()?;

    let token = config
        .digital_ocean_token
        .take()
        .expect("No digital ocean token in config");
    let do_api = Box::new(DigitalOceanApi::new(token));

    let term_handler = AppTerminationHandler::new()?;
    term_handler.setup_exit_panic_hook();

    let updater = Updater::new(config, do_api, term_handler.clone());

    let updater_thread_handle = updater.start_update_loop_detached();
    term_handler.set_updater_thread(updater_thread_handle);
    term_handler.handle_term_signals_gracefully()?;
    Ok(())
}
