use color_eyre::eyre::Result;
use signal_hook::consts::signal::{SIGINT, SIGQUIT, SIGTERM};
use signal_hook::flag;
use signal_hook::iterator::exfiltrator::WithOrigin;
use signal_hook::iterator::SignalsInfo;
use signal_hook::{consts::TERM_SIGNALS, low_level::signal_name};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tracing::{info, trace};

pub struct SignalsHandle {
    handle: signal_hook::iterator::backend::Handle,
}

impl SignalsHandle {
    pub fn new(handle: signal_hook::iterator::backend::Handle) -> Self {
        SignalsHandle { handle }
    }

    pub fn close(&self) {
        self.handle.close();
    }
}

#[derive(Default, Clone)]
pub struct AppTerminationHandler {
    updater_thread: Arc<Mutex<Option<JoinHandle<Result<()>>>>>,
    should_exit_flag: Arc<AtomicBool>,
    signals: Arc<Mutex<Option<SignalsInfo<WithOrigin>>>>,
    signals_handle: Arc<Mutex<Option<SignalsHandle>>>,
}

impl AppTerminationHandler {
    pub fn new() -> Result<Self, std::io::Error> {
        let term_handler = Self::default();

        let signals = TERM_SIGNALS;
        let signals = SignalsInfo::<WithOrigin>::new(signals)?;
        term_handler.set_signals(signals);

        let signals_handle = term_handler
            .signals
            .lock()
            .expect("signals mutex poisoned")
            .as_ref()
            .expect("signals option should exist")
            .handle();
        term_handler.set_signals_handle(SignalsHandle::new(signals_handle));

        Ok(term_handler)
    }

    pub fn set_updater_thread(&self, updater_thread: JoinHandle<Result<()>>) {
        self.updater_thread
            .lock()
            .expect("updater_thread mutex poisoned")
            .replace(updater_thread);
    }

    fn set_signals(&self, signals: SignalsInfo<WithOrigin>) {
        self.signals
            .lock()
            .expect("signals mutex poisoned")
            .replace(signals);
    }

    fn set_signals_handle(&self, signals_handle: SignalsHandle) {
        self.signals_handle
            .lock()
            .expect("signals_handle mutex poisoned")
            .replace(signals_handle);
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit_flag.load(Ordering::SeqCst)
    }

    pub fn set_should_exit(&self) {
        self.should_exit_flag.store(true, Ordering::SeqCst);
    }

    fn unpark_threads(&self) {
        // Unpark the updater thread if we have a handle to it.
        if let Some(updater_thread) = self
            .updater_thread
            .lock()
            .expect("updater thread handle mutex poisoned")
            .as_ref()
        {
            trace!("Unparking updater thread");
            updater_thread.thread().unpark();
        }
    }

    fn notify_threads_to_exit(&self) {}

    pub fn join_threads(&self) -> Result<()> {
        trace!("Joining all thread handles");

        self.updater_thread
            .lock()
            .expect("updater thread handle mutex poisoned")
            .take()
            .expect("updater thread handle should exist")
            .join()
            .expect("updater thread handle join returned an error")?;
        trace!("Updater thread successfully shut down");
        Ok(())
    }

    pub fn notify_exit(&self) {
        // Protect against double mutex lock on updater_thread when joining the thread to the main thread
        // invoking our custom panic handler which then tries to lock the mutex again to unpark the updater_thread.
        if self.should_exit() {
            return;
        }

        trace!("Notifying all threads to exit");
        self.set_should_exit();
        self.notify_threads_to_exit();
        self.unpark_threads();
    }

    pub fn notify_exit_and_stop_signal_handling(&self) {
        self.notify_exit();
        trace!("Stopping signal processing");
        self.signals_handle
            .lock()
            .expect("signals_handle mutex poisoned")
            .as_ref()
            .expect("signals_handle option should exist")
            .close();
    }

    pub fn setup_exit_panic_hook(&self) {
        let orig_hook = std::panic::take_hook();
        let term_handler = self.clone();
        std::panic::set_hook(Box::new(move |panic_info| {
            trace!("Invoked custom panic hook");
            term_handler.notify_exit_and_stop_signal_handling();
            orig_hook(panic_info);
        }));
    }

    pub fn handle_term_signals_gracefully(self) -> Result<()> {
        let mut signals_guard = self.signals.lock().expect("signals mutex poisoned");
        let signals = signals_guard.as_mut().expect("signals option should exist");
        for info in signals {
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
                    assert!(TERM_SIGNALS.contains(&info.signal));
                    info!("Starting process termination due to received signal");
                    self.notify_exit();
                    break;
                }
                _ => unreachable!(),
            }
        }
        self.join_threads()?;

        info!("All threads shut down. Process will now exit");
        Ok(())
    }
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
