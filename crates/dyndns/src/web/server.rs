use color_eyre::eyre::{Result, WrapErr};
use diesel::SqliteConnection;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use tailsome::IntoResult;
use tracing::info;

use crate::config::app_config::AppConfig;
use crate::db::setup::setup_db;
use crate::signal_handlers::AppTerminationHandler;
use crate::web::addresses::{
    print_listener_addresses, socket_acceptor_from_socket_addreses,
    socket_addresses_from_host_and_port,
};
use crate::web::routes::get_final_router;
use crate::web::static_server::print_where_files_are_served_from;

#[derive(Clone)]
pub struct WebServerState {
    pub conn: Arc<Mutex<SqliteConnection>>,
}

pub fn start_web_server_and_wait(term_handler: AppTerminationHandler, config: &AppConfig) {
    if !config.general_options.collect_stats || !config.general_options.enable_web {
        return;
    }

    let (web_exit_tx, web_exit_rx) = tokio::sync::oneshot::channel::<()>();
    term_handler.set_web_exit_tx(web_exit_tx);

    let wait_updater = Arc::new((Mutex::new(false), Condvar::new()));
    let notify_updater = Arc::clone(&wait_updater);

    let web_thread_handle = start_web_server_thread(web_exit_rx, config.clone(), notify_updater);
    term_handler.set_web_thread(web_thread_handle);

    // Wait until the web server has started before returning from the function.
    // Ensures the listening address is printed to stdout without interleaved output from the updater thread.
    let (lock, cvar) = &*wait_updater;
    let mut ready_to_start_updater = lock.lock().expect("wait_updater lock poisoned");
    while !*ready_to_start_updater {
        ready_to_start_updater = cvar
            .wait(ready_to_start_updater)
            .expect("wait_updater lock poisoned while waiting");
    }
}

pub fn start_web_server_thread(
    web_exit_rx: tokio::sync::oneshot::Receiver<()>,
    config: AppConfig,
    notify_updater: Arc<(Mutex<bool>, Condvar)>,
) -> JoinHandle<Result<()>> {
    std::thread::spawn(move || start_web_server_runtime(web_exit_rx, config, notify_updater))
}

pub fn start_web_server_runtime(
    web_exit_rx: tokio::sync::oneshot::Receiver<()>,
    config: AppConfig,
    notify_updater: Arc<(Mutex<bool>, Condvar)>,
) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(start_web_server(web_exit_rx, config, notify_updater))?;
    Ok(())
}

fn notify_updater_server_addresses_printed(notify_updater: Arc<(Mutex<bool>, Condvar)>) {
    // We printed the listening addresses, notify the updater to start.
    let (lock, cvar) = &*notify_updater;
    let mut ready_to_start_updater = lock.lock().expect("notify_updater lock poisoned");
    *ready_to_start_updater = true;
    cvar.notify_one();
}

fn make_web_state(config: AppConfig) -> Result<WebServerState> {
    let db_conn = setup_db(config.general_options.db_path.clone())
        .wrap_err("missing db connection to create web app state")?;
    WebServerState {
        conn: Arc::new(Mutex::new(db_conn)),
    }
    .into_ok()
}

async fn start_web_server(
    web_exit_rx: tokio::sync::oneshot::Receiver<()>,
    config: AppConfig,
    notify_updater: Arc<(Mutex<bool>, Condvar)>,
) -> Result<()> {
    let state = make_web_state(config.clone())?;

    let addrs = socket_addresses_from_host_and_port(
        config.general_options.listen_hostname.as_str(),
        config.general_options.listen_port,
    )?;

    print_listener_addresses(&addrs);
    notify_updater_server_addresses_printed(notify_updater);

    let incoming = socket_acceptor_from_socket_addreses(&addrs)?;
    print_where_files_are_served_from();

    let router = get_final_router(state);

    axum::Server::builder(incoming)
        .serve(router.into_make_service())
        .with_graceful_shutdown(async {
            web_exit_rx.await.ok();
            info!("Web server received signal to shut down");
        })
        .await
        .wrap_err("web server shutdown error")?;

    info!("Web server was shutdown");
    Ok(())
}
