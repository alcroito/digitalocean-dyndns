use clap::ArgMatches;

use crate::cli::get_clap_matches;
use crate::config_consts::BUILD_INFO;

pub struct EarlyConfig {
    clap_matches: ArgMatches<'static>,
}

impl EarlyConfig {
    pub fn get() -> Self {
        let clap_matches = get_clap_matches();
        EarlyConfig { clap_matches }
    }

    pub fn should_print_build_info(&self) -> bool {
        self.clap_matches.is_present(BUILD_INFO)
    }

    pub fn get_clap_matches(&self) -> &ArgMatches<'static> {
        &self.clap_matches
    }
}
