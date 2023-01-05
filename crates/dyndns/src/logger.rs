use anyhow::{Context, Result};
use once_cell::sync::OnceCell;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter, fmt, prelude::*, reload, Registry};

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
        .map_err(|_| anyhow::anyhow!("Could not save tracing filter reload handle"))
}

pub fn setup_logger(log_level: &tracing::Level) -> Result<()> {
    // Set the logging level that was read from the config file.
    TRACING_FILTER_RELOAD_HANDLE
        .get()
        .ok_or_else(|| anyhow::anyhow!("Could not load tracing filtering reload handle"))?
        .modify(|filter| *filter = LevelFilter::from_level(*log_level))
        .map_err(|e| anyhow::anyhow!(e))
        .context("Could not modify the global tracing filter")
}
