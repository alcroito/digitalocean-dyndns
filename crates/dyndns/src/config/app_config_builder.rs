use crate::cli::ClapAllArgs;

use super::app_config::{
    AppConfig, AppConfigInner, Domain, DomainRecord, Domains, GeneralOptions,
    GeneralOptionsDefaults, SimpleModeDomainConfig,
};
use super::consts::*;
use super::early::EarlyConfig;

use clap::ArgMatches;
use color_eyre::eyre::{bail, eyre, Result, WrapErr};
use figment::Figment;
use tracing::trace;

fn get_default_config_path() -> &'static str {
    "./config/do_ddns.toml"
}

fn file_is_readable(path: &str) -> bool {
    std::fs::File::open(path).is_ok()
}

fn make_env_var_from_key(key: &str) -> String {
    format!("{}{}", ENV_VAR_PREFIX, key.to_ascii_uppercase())
}

fn get_config_path_candidates(clap_matches: &ArgMatches) -> Vec<String> {
    let mut candidates = vec![];

    // First check env.
    if let Ok(v) = std::env::var(make_env_var_from_key(CONFIG_KEY)) {
        candidates.push(v);
    }

    // Then check command line.
    if let Some(v) = clap_matches
        .get_one::<String>(CONFIG_KEY)
        .map(|s| s.as_str())
    {
        candidates.push(v.to_owned());
    }

    // Finally check the default path.
    candidates.push(get_default_config_path().to_owned());
    candidates
}

fn get_config_path_from_candidates(candidates: &[String]) -> Option<String> {
    trace!(
        "Looking for config file. Checking following paths: {:#?}",
        candidates
    );
    let config_file = candidates
        .iter()
        .find(|path| {
            let readable = file_is_readable(path);
            let canonical_path = std::fs::canonicalize(path);
            let canonical_path = match canonical_path {
                Ok(path) => format!("{}", path.display()),
                Err(e) => format!("Error: {}", eyre!(e)),
            };
            trace!(
                "Checking if config file exists and is readable:
  file: {path}
  canonical path: {canonical_path}
  readable: {readable}"
            );
            readable
        })
        .map(|path| path.to_owned());

    if let Some(path) = &config_file {
        trace!("Final config file chosen: {}", path);
    } else {
        trace!("No valid config file found. Make sure required options are set via command line options or environment variables.");
    };
    config_file
}

fn get_config_path(clap_matches: &ArgMatches) -> Option<String> {
    let candidates = get_config_path_candidates(clap_matches);
    get_config_path_from_candidates(&candidates)
}

pub fn config_with_args(early_config: &EarlyConfig) -> Result<AppConfig> {
    let clap_matches = early_config.get_clap_matches();
    let config_file_path = get_config_path(clap_matches);
    let config_builder = AppConfigBuilder::new(Some(clap_matches), config_file_path);
    let config = config_builder
        .inspect_err(|_e| {
            tracing::error!(
                "Failed to initialize configuration system. Will exit shortly with error details."
            );
        })?
        .build()
        .inspect_err(|_e| {
            tracing::error!(
                "Failed to configure the application. Will exit shortly with error details."
            );
        })
        .wrap_err(eyre!("Failed to configure the application"))?;
    Ok(config)
}

fn get_advanced_mode_domains(builder: &AppConfigBuilder) -> Result<Domains> {
    let domains: Domains = builder
        .figment
        .extract_inner(DOMAINS_CONFIG_KEY)
        .map_err(|e| eyre!(e).wrap_err("Failed to parse 'advanced mode' domain section"))?;
    Ok(domains)
}

pub struct AppConfigBuilder {
    figment: Figment,
}

impl AppConfigBuilder {
    pub fn new(
        clap_matches: Option<&ArgMatches>,
        config_file_path: Option<String>,
    ) -> Result<Self> {
        let figment = Self::prepare_figmment(clap_matches, config_file_path.as_deref())?;

        let builder = AppConfigBuilder { figment };

        Ok(builder)
    }

    fn prepare_figmment(
        clap_matches: Option<&ArgMatches>,
        config_file_path: Option<&str>,
    ) -> Result<Figment> {
        use figment::providers::{Env, Format, Serialized, Toml};
        use figment_file_provider_adapter::FileAdapter;

        let mut figment =
            Figment::new().merge(Serialized::defaults(GeneralOptionsDefaults::default()));

        if let Some(config_file_path) = config_file_path {
            figment = figment.merge(Toml::file(config_file_path));
        }

        if let Some(clap_matches) = clap_matches {
            let clap_args = ClapAllArgs::parse_and_process(clap_matches)?;
            let wrapped_clap_figment = FileAdapter::wrap(Serialized::defaults(clap_args))
                .with_suffix("_path")
                .only(&[DIGITAL_OCEAN_TOKEN_PATH]);
            figment = figment.merge(wrapped_clap_figment);
        }

        figment = figment.merge(Env::prefixed(ENV_VAR_PREFIX));

        Ok(figment)
    }

    fn build_simple_mode_domain_config_values(&self) -> Result<(String, String)> {
        let config: SimpleModeDomainConfig = self.figment.extract()?;

        let hostname_part = match (config.subdomain_to_update, config.update_domain_root) {
            (Some(subdomain_to_update), None) => subdomain_to_update,
            (None, Some(update_domain_root)) => {
                if update_domain_root {
                    "@".to_owned()
                } else {
                    bail!("Please provide a subdomain to update")
                }
            }
            (None, None) => {
                bail!("Neither 'subdomain to update' nor 'update domain root' options were set. Please provide one.")
            }
            (Some(_), Some(_)) => {
                bail!("Both 'subdomain to update' and 'update domain root' options were set. Please provide only one option")
            }
        };
        Ok((config.domain_root, hostname_part))
    }

    fn simple_mode_domains_as_records(config: Result<(String, String)>) -> Result<Domains> {
        let config = config?;
        let domains = Domains {
            domains: vec![Domain {
                name: config.0,
                records: vec![DomainRecord {
                    record_type: "A".to_owned(),
                    name: config.1,
                }],
            }],
        };
        Ok(domains)
    }

    fn build_domains(&self) -> Result<Domains> {
        let simple_mode_domains = AppConfigBuilder::simple_mode_domains_as_records(
            self.build_simple_mode_domain_config_values(),
        );
        let advanced_mode_domains = get_advanced_mode_domains(self);
        let domains = match (simple_mode_domains, advanced_mode_domains) {
            (Ok(simple_mode_domains), Err(_)) => simple_mode_domains,
            (Err(_), Ok(advanced_mode_domains)) => advanced_mode_domains,
            (Err(e1), Err(e2)) => {
                let e1 = e1.wrap_err("Simple mode configuration error");
                let e2 = e2.wrap_err("Advanced mode configuration error");
                let e = format!("{e1:#}\n{e2:#}");
                return Err(eyre!(e)
                    .wrap_err("No valid domain configuration found with either supported modes"));
            }
            (Ok(_), Ok(_)) => {
                bail!("Both simple and advanced config modes settings were specified. Please use only one mode")
            }
        };
        Ok(domains)
    }

    fn build_general_options(&self) -> Result<GeneralOptions> {
        let general_options: GeneralOptions = self.figment.extract()?;

        if !general_options.ipv4 && !general_options.ipv6 {
            bail!("At least one kind of ip family support needs to be enabled, both are disabled.");
        }

        Ok(general_options)
    }

    pub fn build(&self) -> Result<AppConfig> {
        let general_options = self.build_general_options()?;

        let domains = self.build_domains()?;

        let config = AppConfig::new(AppConfigInner {
            domains,
            general_options,
        });
        Ok(config)
    }
}
