use crate::config::Config;
use crate::domain_record_api::digital_ocean::DigitalOceanApi;
use crate::logger::setup_logger;
use crate::signal_handlers::{handle_term_signals_gracefully, setup_forceful_term_signal_handling};
use crate::updater::Updater;
use anyhow::Result;

pub fn start_daemon(mut config: Config) -> Result<()> {
    setup_logger(&config.log_level)?;
    setup_forceful_term_signal_handling()?;

    let token = config
        .digital_ocean_token
        .take()
        .expect("No digital ocean token in config");
    let do_api = Box::new(DigitalOceanApi::new(token));
    let updater = Updater::new(config, do_api);
    let exit_flag = updater.exit_flag();
    let app_thread = updater.start_update_loop_detached();

    handle_term_signals_gracefully(app_thread, exit_flag)?;
    Ok(())
}
