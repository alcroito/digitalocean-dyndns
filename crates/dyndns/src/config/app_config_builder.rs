use super::app_config::{
    AppConfig, AppConfigInner, Domain, DomainRecord, Domains, GeneralOptions, UpdateInterval,
};
use super::config_builder::{make_env_var_from_key, ValueBuilder};
use super::consts::*;
use super::early::EarlyConfig;
use crate::token::SecretDigitalOceanToken;
use color_eyre::eyre::{bail, eyre, Result, WrapErr};

use clap::ArgMatches;
use tracing::trace;

fn get_default_config_path() -> &'static str {
    "./config/do_ddns.toml"
}

fn read_config_map(config_path: &str) -> Result<toml::Value> {
    let config = std::fs::read_to_string(config_path)
        .wrap_err(format!("Failed to read config file: {config_path}"))?;
    let config =
        toml::from_str(&config).wrap_err(format!("Failed to parse config file: {config_path}"))?;
    Ok(config)
}

fn file_is_readable(path: &str) -> bool {
    std::fs::File::open(path).is_ok()
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

    match &config_file {
        Some(path) => trace!("Final config file chosen: {}", path),
        None => trace!("No valid config file found. Make sure required options are set via command line options or environment variables."),
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
        .build()
        .map_err(|e| {
            tracing::error!(
                "Failed to configure the application. Will exit shortly with error details."
            );
            e
        })
        .wrap_err(eyre!("Failed to configure the application"))?;
    Ok(config)
}

fn get_advanced_mode_domains(table: Option<&toml::value::Table>) -> Result<Domains> {
    let domains = table
        .ok_or_else(|| {
            eyre!("No config contents found while retrieving 'advanced mode' domains section")
        })?
        .get(DOMAINS_CONFIG_KEY)
        .ok_or_else(|| eyre!("No 'advanced mode' domains section found in config"))?
        .clone()
        .try_into::<Domains>()
        .map_err(|e| eyre!(e).wrap_err("Failed to parse 'advanced mode' domain section"))?;
    Ok(domains)
}

pub struct AppConfigBuilder<'clap> {
    clap_matches: Option<&'clap ArgMatches>,
    toml_table: Option<toml::value::Table>,
    domain_root: Option<String>,
    subdomain_to_update: Option<String>,
    update_domain_root: Option<bool>,
    update_interval: Option<UpdateInterval>,
    digital_ocean_token: Option<SecretDigitalOceanToken>,
    log_level: Option<tracing::Level>,
    dry_run: Option<bool>,
    ipv6: Option<bool>,
    db_path: Option<String>,
}

impl<'clap> AppConfigBuilder<'clap> {
    pub fn new(clap_matches: Option<&'clap ArgMatches>, config_file_path: Option<String>) -> Self {
        fn get_config(config_file_path: &str) -> Result<toml::value::Table> {
            let toml_value = read_config_map(config_file_path)?;
            let toml_table = match toml_value {
                toml::value::Value::Table(table) => table,
                _ => bail!("Failed to deserialize config file"),
            };
            Ok(toml_table)
        }

        let mut toml_table = None;
        if let Some(config_file_path) = config_file_path {
            toml_table = get_config(&config_file_path)
                .map_err(|e| {
                    tracing::error!("{:#}", e);
                    e
                })
                .ok();
        }

        AppConfigBuilder {
            clap_matches,
            toml_table,
            domain_root: None,
            update_domain_root: None,
            subdomain_to_update: None,
            update_interval: None,
            digital_ocean_token: None,
            log_level: None,
            dry_run: None,
            ipv6: None,
            db_path: None,
        }
    }

    pub fn set_domain_root(&mut self, value: String) -> &mut Self {
        self.domain_root = Some(value);
        self
    }

    pub fn set_subdomain_to_update(&mut self, value: String) -> &mut Self {
        self.subdomain_to_update = Some(value);
        self
    }

    pub fn set_update_domain_root(&mut self, value: bool) -> &mut Self {
        self.update_domain_root = Some(value);
        self
    }

    pub fn set_update_interval(&mut self, value: UpdateInterval) -> &mut Self {
        self.update_interval = Some(value);
        self
    }

    pub fn set_digital_ocean_token(&mut self, value: SecretDigitalOceanToken) -> &mut Self {
        self.digital_ocean_token = Some(value);
        self
    }

    pub fn set_log_level(&mut self, value: tracing::Level) -> &mut Self {
        self.log_level = Some(value);
        self
    }

    pub fn set_dry_run(&mut self, value: bool) -> &mut Self {
        self.dry_run = Some(value);
        self
    }

    fn build_simple_mode_domain_config_values(&self) -> Result<(String, String)> {
        let domain_root = ValueBuilder::new(DOMAIN_ROOT)
            .with_value(self.domain_root.clone())
            .with_env_var_name()
            .with_clap(self.clap_matches)
            .with_config_value(self.toml_table.as_ref())
            .build()?;

        let subdomain_to_update = ValueBuilder::new(SUBDOMAIN_TO_UPDATE)
            .with_value(self.subdomain_to_update.clone())
            .with_env_var_name()
            .with_clap(self.clap_matches)
            .with_config_value(self.toml_table.as_ref())
            .build();

        let update_domain_root = ValueBuilder::new(UPDATE_DOMAIN_ROOT)
            .with_value(self.update_domain_root)
            .with_env_var_name()
            .with_clap_bool(self.clap_matches)
            .with_config_value(self.toml_table.as_ref())
            .build();

        let hostname_part = match (subdomain_to_update, update_domain_root) {
            (Ok(subdomain_to_update), Err(_)) => subdomain_to_update,
            (Err(_), Ok(update_domain_root)) => {
                if update_domain_root {
                    "@".to_owned()
                } else {
                    bail!("Please provide a subdomain to update")
                }
            }
            (Err(e1), Err(e2)) => {
                let e = format!("{e1:#}\n{e2:#}");
                return Err(eyre!(e).wrap_err("No valid domain to update found"));
            }
            (Ok(_), Ok(_)) => {
                bail!("Both 'subdomain to update' and 'update domain root' options were set. Please provide only one option")
            }
        };
        Ok((domain_root, hostname_part))
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
        let advanced_mode_domains = get_advanced_mode_domains(self.toml_table.as_ref());
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
        let update_interval = ValueBuilder::new(UPDATE_INTERVAL)
            .with_value(self.update_interval.clone())
            .with_env_var_name()
            .with_clap(self.clap_matches)
            .with_config_value(self.toml_table.as_ref())
            .with_default(UpdateInterval::default())
            .build()?;

        let mut builder = ValueBuilder::new(DIGITAL_OCEAN_TOKEN);
        builder
            .with_value(self.digital_ocean_token.clone())
            .with_env_var_name()
            .with_clap(self.clap_matches)
            .with_config_value(self.toml_table.as_ref());
        if let Some(clap_matches) = self.clap_matches {
            let from_file = clap_matches
                .get_one::<String>(DIGITAL_OCEAN_TOKEN_PATH)
                .map(|s| s.as_str());
            if let Some(from_file) = from_file {
                builder.with_single_line_from_file(from_file);
            }
        }

        let digital_ocean_token: SecretDigitalOceanToken = builder.build()?;

        let log_level = ValueBuilder::new(SERVICE_LOG_LEVEL)
            .with_value(self.log_level)
            .with_env_var_name()
            .with_clap_occurences(
                self.clap_matches,
                LOG_LEVEL_VERBOSITY_SHORT,
                Box::new(|count| match count {
                    0 => None,
                    1 => Some(tracing::Level::DEBUG),
                    2 => Some(tracing::Level::TRACE),
                    _ => Some(tracing::Level::TRACE),
                }),
            )
            .with_config_value(self.toml_table.as_ref())
            .with_default(tracing::Level::INFO)
            .build()?;

        let dry_run = ValueBuilder::new(DRY_RUN)
            .with_value(self.dry_run)
            .with_env_var_name()
            .with_clap_bool(self.clap_matches)
            .with_config_value(self.toml_table.as_ref())
            .with_default(false)
            .build()?;

        // TODO: Figure out the bool clap cli issue where it is always true even if it's
        // specified as false.
        let ipv4 = true;

        let ipv6 = ValueBuilder::new(IPV6_SUPPORT)
            .with_value(self.ipv6)
            .with_env_var_name()
            .with_clap_bool(self.clap_matches)
            .with_config_value(self.toml_table.as_ref())
            .with_default(false)
            .build()?;

        if !ipv4 && !ipv6 {
            bail!("At least one kind of ip family support needs to be enabled, both are disabled.");
        }

        let collect_stats = ValueBuilder::new(COLLECT_STATS)
            .with_env_var_name()
            .with_clap_bool(self.clap_matches)
            .with_config_value(self.toml_table.as_ref())
            .with_default(false)
            .build()?;

        let db_path = ValueBuilder::new(DB_PATH)
            .with_value(self.db_path.clone())
            .with_env_var_name()
            .with_clap(self.clap_matches)
            .with_config_value(self.toml_table.as_ref())
            .build()
            .ok()
            .map(|db_path| std::path::PathBuf::from(&db_path));

        let enable_web = ValueBuilder::new(ENABLE_WEB)
            .with_env_var_name()
            .with_clap_bool(self.clap_matches)
            .with_config_value(self.toml_table.as_ref())
            .with_default(false)
            .build()?;

        let listen_hostname: String = ValueBuilder::new(LISTEN_HOSTNAME)
            .with_env_var_name()
            .with_clap(self.clap_matches)
            .with_config_value(self.toml_table.as_ref())
            .with_default("localhost".to_owned())
            .build()?;

        let listen_port = ValueBuilder::new(LISTEN_PORT)
            .with_env_var_name()
            .with_clap(self.clap_matches)
            .with_config_value(self.toml_table.as_ref())
            .with_default(8095_u16)
            .build()?;

        let general_options = GeneralOptions {
            update_interval,
            digital_ocean_token,
            log_level,
            dry_run,
            ipv4,
            ipv6,
            collect_stats,
            db_path,
            enable_web,
            listen_hostname,
            listen_port,
        };
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
