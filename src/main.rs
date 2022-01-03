use anyhow::{anyhow, Result};
use do_ddns::build_info::print_build_info_if_requested;
use do_ddns::cli::get_clap_matches;
use do_ddns::config::Config;
use do_ddns::config_builder::config_with_args;
use do_ddns::domain_record_api::digital_ocean::DigitalOceanApi;
use do_ddns::logger::{setup_early_logger, setup_logger};
use do_ddns::signal_handlers::{
    handle_term_signals_gracefully, setup_forceful_term_signal_handling,
};
use do_ddns::updater::Updater;
use log::error;
fn main() -> Result<()> {
    setup_early_logger()?;

    main_impl().map_err(|e| {
        error!("{:?}", e);
        anyhow!("something failed")
    })
}

fn main_impl() -> Result<()> {
    let clap_matches = get_clap_matches();

    if print_build_info_if_requested(&clap_matches) {
        return Ok(());
    }

    let config = config_with_args(&clap_matches)?;
    start_daemon(config)?;
    Ok(())
}

fn start_daemon(mut config: Config) -> Result<()> {
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
