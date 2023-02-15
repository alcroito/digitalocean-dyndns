use clap::ArgMatches;

use super::consts::BUILD_INFO;
use crate::cli::get_cli_args;

pub struct EarlyConfig {
    clap_matches: ArgMatches,
}

impl EarlyConfig {
    pub fn get() -> Self {
        let clap_matches = get_cli_args();
        EarlyConfig { clap_matches }
    }

    pub fn should_print_build_info(&self) -> bool {
        self.clap_matches.get_flag(BUILD_INFO)
    }

    pub fn get_clap_matches(&self) -> &ArgMatches {
        &self.clap_matches
    }
}
