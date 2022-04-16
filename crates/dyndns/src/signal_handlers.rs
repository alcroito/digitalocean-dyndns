use anyhow::Result;
use log::info;
use signal_hook::consts::signal::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook::flag;
use signal_hook::iterator::exfiltrator::WithOrigin;
use signal_hook::iterator::SignalsInfo;
use signal_hook::{consts::TERM_SIGNALS, low_level::signal_name};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub fn handle_term_signals_gracefully(
    app_thread: std::thread::JoinHandle<Result<()>>,
    exit_flag: Arc<AtomicBool>,
) -> Result<()> {
    let signals = TERM_SIGNALS;
    let mut signals = SignalsInfo::<WithOrigin>::new(signals)?;
    for info in &mut signals {
        let killer_pid = info
            .process
            .map(|p| format!(" by pid {}", p.pid))
            .unwrap_or_else(|| "".to_owned());
        info!(
            "Received signal {}{}",
            signal_name(info.signal).expect("Empty signal name"),
            killer_pid
        );
        match info.signal {
            SIGTERM | SIGQUIT | SIGINT => {
                info!("Starting process termination");
                assert!(TERM_SIGNALS.contains(&info.signal));
                exit_flag.store(true, Ordering::SeqCst);
                app_thread.thread().unpark();
                break;
            }
            _ => unreachable!(),
        }
    }
    app_thread.join().unwrap()
}

pub fn setup_forceful_term_signal_handling() -> Result<()> {
    // Double Ctrl-C terminator.
    let term_now = Arc::new(AtomicBool::new(false));
    for sig in TERM_SIGNALS {
        flag::register_conditional_shutdown(*sig, 1, Arc::clone(&term_now))?;
        flag::register(*sig, Arc::clone(&term_now))?;
    }
    Ok(())
}
