use anyhow::Result;
use do_ddns::cli::get_clap_matches;
use do_ddns::config::Config;
use do_ddns::config_builder::config_with_args;
use do_ddns::domain_updater::DigitalOceanUpdater;
use do_ddns::logger::{setup_early_logger, setup_logger};
use do_ddns::signal_handlers::{
    handle_term_signals_gracefully, setup_forceful_term_signal_handling,
};

fn main() -> Result<()> {
    setup_early_logger()?;
    let clap_matches = get_clap_matches();
    let config = config_with_args(&clap_matches)?;
    start_daemon(config)
}

fn start_daemon(config: Config) -> Result<()> {
    setup_logger(&config.log_level)?;
    setup_forceful_term_signal_handling()?;
    let updater = DigitalOceanUpdater::new(config);
    let exit_flag = updater.exit_flag();
    let app_thread = updater.start_update_loop_detached();

    handle_term_signals_gracefully(app_thread, exit_flag)?;
    Ok(())
}
