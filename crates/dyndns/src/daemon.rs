use crate::domain_record_api::digital_ocean::DigitalOceanApi;
use crate::global_state::GlobalState;
use crate::logger::setup_logger;
use crate::signal_handlers::{setup_forceful_term_signal_handling, AppTerminationHandler};
use crate::updater::Updater;
use color_eyre::eyre::Result;

#[cfg(feature = "web")]
use crate::web::server::start_web_server_and_wait;

pub fn start_daemon(global_state: GlobalState) -> Result<()> {
    setup_logger(&global_state.config.general_options.log_level)?;
    setup_forceful_term_signal_handling()?;

    let do_api = Box::new(DigitalOceanApi::new(global_state.config.clone()));

    let term_handler = AppTerminationHandler::new()?;
    term_handler.setup_exit_panic_hook();

    #[cfg(feature = "web")]
    start_web_server_and_wait(term_handler.clone(), &global_state.config);

    let updater = Updater::new(global_state, do_api, term_handler.clone());
    let updater_thread_handle = updater.start_update_loop_detached();
    term_handler.set_updater_thread(updater_thread_handle);
    term_handler.handle_term_signals_gracefully()?;
    Ok(())
}
