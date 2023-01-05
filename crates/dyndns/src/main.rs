use color_eyre::eyre::Result;
use do_ddns::commands::{decide_command, handle_command};
use do_ddns::config_early::EarlyConfig;
use do_ddns::logger::{setup_early_logger, EyreSpanTraceWorkaroundGuard};

fn main() -> Result<()> {
    setup_error_reporting_and_logger()?;
    main_impl()
}

fn setup_error_reporting_and_logger() -> Result<()> {
    EyreSpanTraceWorkaroundGuard::run(|| {
        color_eyre::install()?;
        setup_early_logger()?;
        Ok(())
    })
}

fn main_impl() -> Result<()> {
    let early_config = EarlyConfig::get();
    let command = decide_command(&early_config);
    handle_command(&early_config, &command)?;
    Ok(())
}
