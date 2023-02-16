use crate::config::app_config::AppConfig;
use crate::config::app_config_builder::config_with_args;
use crate::config::early::EarlyConfig;
use color_eyre::eyre::Result;

pub struct GlobalState {
    pub config: AppConfig,
}

impl GlobalState {
    pub fn new(early_config: &EarlyConfig) -> Result<Self> {
        let config = config_with_args(early_config)?;
        let global_state = GlobalState { config };
        Ok(global_state)
    }
}
