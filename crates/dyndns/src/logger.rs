use color_eyre::eyre::{eyre, Result, WrapErr};
use once_cell::sync::OnceCell;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter, fmt, prelude::*, reload, Registry};

const RUST_SPANTRACE_KEY: &str = "RUST_SPANTRACE";

static TRACING_FILTER_RELOAD_HANDLE: OnceCell<reload::Handle<filter::LevelFilter, Registry>> =
    OnceCell::new();

pub fn setup_early_logger() -> Result<()> {
    // Initialize tracing_suscriber with the default formatter and a runtime modifiable filter.
    // Set the max level to TRACE, to log which log file is found.
    // Store the reloadable filter handle in a static variable, which will be used to set the
    // final logging level after the config file is loded.
    let filter = filter::LevelFilter::TRACE;
    let (filter, reload_handle) = reload::Layer::new(filter);
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::Layer::default())
        .init();
    TRACING_FILTER_RELOAD_HANDLE
        .set(reload_handle)
        .map_err(|_| eyre!("Could not save tracing filter reload handle"))
}

pub fn setup_logger(log_level: &tracing::Level) -> Result<()> {
    // Set the logging level that was read from the config file.
    TRACING_FILTER_RELOAD_HANDLE
        .get()
        .ok_or_else(|| eyre!("Could not load tracing filtering reload handle"))?
        .modify(|filter| *filter = LevelFilter::from_level(*log_level))
        .map_err(|e| eyre!(e))
        .wrap_err("Could not modify the global tracing filter")
}

pub struct EyreSpanTraceWorkaroundGuard;

impl EyreSpanTraceWorkaroundGuard {
    pub fn run<F>(mut f: F) -> Result<()>
    where
        F: FnMut() -> Result<()>,
    {
        // Work around https://github.com/yaahc/color-eyre/issues/110
        std::env::set_var(RUST_SPANTRACE_KEY, "0");
        f()
    }
}

impl Drop for EyreSpanTraceWorkaroundGuard {
    fn drop(&mut self) {
        std::env::set_var(RUST_SPANTRACE_KEY, "1");
    }
}

struct ColorEyreGuard(());
static INIT_COLOR_EYRE: OnceCell<ColorEyreGuard> = OnceCell::new();

pub fn init_color_eyre() {
    INIT_COLOR_EYRE.get_or_init(|| {
        color_eyre::install().expect("Failed to initialize color_eyre");
        ColorEyreGuard(())
    });
}
