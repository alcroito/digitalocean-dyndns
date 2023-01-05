use anyhow::{anyhow, Result};
use do_ddns::commands::{decide_command, handle_command};
use do_ddns::config_early::EarlyConfig;
use do_ddns::logger::setup_early_logger;
use tracing::error;
fn main() -> Result<()> {
    setup_early_logger()?;

    main_impl().map_err(|e| {
        error!("{:?}", e);
        anyhow!("something failed")
    })
}

fn main_impl() -> Result<()> {
    let early_config = EarlyConfig::get();
    let command = decide_command(&early_config);
    handle_command(&early_config, &command)?;
    Ok(())
}
