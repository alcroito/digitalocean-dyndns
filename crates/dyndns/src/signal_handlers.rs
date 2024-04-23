use color_eyre::eyre::Result;
use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGQUIT, SIGTERM};
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use signal_hook::iterator::exfiltrator::WithOrigin;
use signal_hook::iterator::SignalsInfo;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[cfg(feature = "web")]
use tokio::sync::oneshot::Sender;

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
    web_thread: Arc<Mutex<Option<JoinHandle<Result<()>>>>>,

    #[cfg(feature = "web")]
    web_exit_tx: Arc<Mutex<Option<Sender<()>>>>,
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

    pub fn set_web_thread(&self, web_thread: JoinHandle<Result<()>>) {
        self.web_thread
            .lock()
            .expect("web_thread mutex poisoned")
            .replace(web_thread);
    }

    #[cfg(feature = "web")]
    pub fn set_web_exit_tx(&self, web_exit_tx: Sender<()>) {
        self.web_exit_tx
            .lock()
            .expect("web_exit_tx mutex poisoned")
            .replace(web_exit_tx);
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
            info!("unparking updater thread");
            updater_thread.thread().unpark();
        }

        #[cfg(feature = "web")]
        {
            // Unpark the web thread if we have a handle to it.
            if let Some(web_thread) = self
                .web_thread
                .lock()
                .expect("web thread handle mutex poisoned")
                .as_ref()
            {
                info!("unparking web thread");
                web_thread.thread().unpark();
            }
        }
    }

    fn notify_threads_to_exit(&self) {
        #[cfg(feature = "web")]
        {
            if let Some(web_exit_tx) = self
                .web_exit_tx
                .lock()
                .expect("web_exit_tx mutex poisoned")
                .take()
            {
                if web_exit_tx.send(()).is_err() {
                    info!("Failed to message the web server runtime to shutdown, the receiver dropped.");
                }
            }
        }
    }

    pub fn join_threads(&self) -> Result<()> {
        trace!("Joining all threads");

        #[cfg(feature = "web")]
        {
            if self
                .web_thread
                .lock()
                .expect("web thread handle mutex poisoned")
                .is_some()
            {
                self.web_thread
                    .lock()
                    .expect("web thread handle mutex poisoned")
                    .take()
                    .expect("web thread handle should exist")
                    .join()
                    .expect("web thread join panicked")?;
                trace!("Web server thread successfully shut down");
            }
        }

        self.updater_thread
            .lock()
            .expect("updater thread handle mutex poisoned")
            .take()
            .expect("updater thread handle should exist")
            .join()
            .expect("updater thread join panicked")?;
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
            let killer_pid_message = info
                .process
                .map(|p| format!(" from pid: {}", p.pid))
                .unwrap_or_else(|| "".to_owned());

            let signal_name = signal_hook::low_level::signal_name(info.signal)
                .map(|s| s.to_owned())
                .unwrap_or_else(|| {
                    info!("Can't find human readable name for signal: {}", info.signal);
                    info.signal.to_string()
                });

            info!("Received signal: {}{}", signal_name, killer_pid_message);
            match info.signal {
                SIGHUP => {
                    info!("Ignoring signal because config reloading is not yet supported")
                }
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
