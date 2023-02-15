use crate::build_info::print_build_info;
use crate::config::app_config_builder::config_with_args;
use crate::config::early::EarlyConfig;
use crate::daemon::start_daemon;
use color_eyre::eyre::Result;

pub enum Command {
    PrintBuildInfo,
    StartDaemon,
}

pub fn decide_command(early_config: &EarlyConfig) -> Command {
    if early_config.should_print_build_info() {
        Command::PrintBuildInfo
    } else {
        Command::StartDaemon
    }
}

pub fn handle_command(early_config: &EarlyConfig, command: &Command) -> Result<()> {
    match command {
        Command::PrintBuildInfo => {
            print_build_info();
        }
        Command::StartDaemon => {
            let config = config_with_args(early_config)?;
            start_daemon(config)?;
        }
    };
    Ok(())
}
