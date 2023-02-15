use super::app_config::{Config, Domain, DomainRecord, Domains, UpdateInterval};
use super::consts::*;
use super::early::EarlyConfig;
use crate::token::SecretDigitalOceanToken;
use crate::types::ValueFromStr;
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

pub fn make_env_var_from_key(key: &str) -> String {
    format!("{}{}", ENV_VAR_PREFIX, key.to_ascii_uppercase())
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

pub fn config_with_args(early_config: &EarlyConfig) -> Result<Config> {
    let clap_matches = early_config.get_clap_matches();
    let config_file_path = get_config_path(clap_matches);
    let config_builder = Builder::new(Some(clap_matches), config_file_path);
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

type OccurencesFn<T> = Box<dyn FnMut(u64) -> Option<T>>;
pub struct ValueBuilder<'clap, 'toml, T> {
    key: String,
    value: Option<T>,
    env_var_name: Option<String>,
    clap_option: Option<(&'clap ArgMatches, String)>,
    clap_option_bool: Option<(&'clap ArgMatches, String)>,
    clap_occurrences_option: Option<(&'clap ArgMatches, String, OccurencesFn<T>)>,
    file_path: Option<String>,
    config_value: Option<(&'toml toml::value::Table, String)>,
    default_value: Option<T>,
}

impl<'clap, 'toml, T: ValueFromStr> ValueBuilder<'clap, 'toml, T> {
    pub fn new(key: &str) -> Self {
        ValueBuilder {
            key: key.to_owned(),
            value: None,
            env_var_name: None,
            clap_option: None,
            clap_option_bool: None,
            clap_occurrences_option: None,
            file_path: None,
            config_value: None,
            default_value: None,
        }
    }

    pub fn with_env_var_name(&mut self) -> &mut Self {
        let env_var_name = make_env_var_from_key(&self.key.to_ascii_uppercase());
        self.env_var_name = Some(env_var_name);
        self
    }

    pub fn with_clap(&mut self, arg_matches: Option<&'clap ArgMatches>) -> &mut Self {
        if let Some(arg_matches) = arg_matches {
            self.clap_option = Some((arg_matches, self.key.to_owned()));
        }
        self
    }

    pub fn with_clap_bool(&mut self, arg_matches: Option<&'clap ArgMatches>) -> &mut Self {
        if let Some(arg_matches) = arg_matches {
            self.clap_option_bool = Some((arg_matches, self.key.to_owned()));
        }
        self
    }

    pub fn with_clap_occurences(
        &mut self,
        arg_matches: Option<&'clap ArgMatches>,
        key: &str,
        clap_fn: OccurencesFn<T>,
    ) -> &mut Self {
        if let Some(arg_matches) = arg_matches {
            self.clap_occurrences_option = Some((arg_matches, key.to_owned(), clap_fn));
        }
        self
    }

    pub fn with_single_line_from_file(&mut self, file_path: &str) -> &mut Self {
        self.file_path = Some(file_path.to_owned());
        self
    }

    pub fn with_config_value(&mut self, toml_map: Option<&'toml toml::value::Table>) -> &mut Self {
        if let Some(toml_map) = toml_map {
            self.config_value = Some((toml_map, self.key.to_owned()));
        }
        self
    }

    pub fn with_default(&mut self, default_value: T) -> &mut Self {
        self.default_value = Some(default_value);
        self
    }

    pub fn with_value(&mut self, value: Option<T>) -> &mut Self {
        self.value = value;
        self
    }

    fn try_from_env(&mut self) -> &mut Self {
        if self.value.is_some() {
            return self;
        }
        if let Some(ref env_var_name) = self.env_var_name {
            let env_res = std::env::var(env_var_name);
            if let Ok(value) = env_res {
                let parsed_res = ValueFromStr::from_str(value.as_ref());
                if let Ok(value) = parsed_res {
                    self.value = Some(value);
                }
            }
        }

        self
    }

    fn try_from_clap(&mut self) -> &mut Self {
        if self.value.is_some() {
            return self;
        }

        if let Some((arg_matches, ref option_name)) = self.clap_option {
            let clap_value = arg_matches
                .get_one::<String>(option_name)
                .map(|s| s.as_str());
            if let Some(value) = clap_value {
                let parsed_res = ValueFromStr::from_str(value);
                if let Ok(value) = parsed_res {
                    self.value = Some(value);
                }
            }
        }

        self
    }

    fn try_from_clap_bool(&mut self) -> &mut Self {
        if self.value.is_some() {
            return self;
        }

        if let Some((arg_matches, ref option_name)) = self.clap_option_bool {
            let clap_value = arg_matches
                .get_one::<String>(option_name)
                .map(|s| s.as_str());
            if let Some(value) = clap_value {
                // FIXME: Ugly workaround to avoid generic type errors trying to assign bool
                // directly. Should rethink.
                let manually_parsed_res = value.parse::<bool>();
                let parsed_res = ValueFromStr::from_str(value);
                if let (Ok(value), Ok(bool_value)) = (parsed_res, manually_parsed_res) {
                    if bool_value {
                        self.value = Some(value);
                    }
                }
            }
        }

        self
    }

    fn try_from_clap_occurences(&mut self) -> &mut Self {
        if self.value.is_some() {
            return self;
        }

        if let Some((arg_matches, ref option_name, ref mut clap_fn)) = self.clap_occurrences_option
        {
            let occurences_value = arg_matches.get_count(option_name);
            self.value = clap_fn(occurences_value as u64);
        }

        self
    }

    fn try_from_file_line(&mut self) -> &mut Self {
        if self.value.is_some() {
            return self;
        }

        if let Some(ref file_path) = self.file_path {
            let line = std::fs::read_to_string(file_path);
            if let Ok(line) = line {
                let value = line.trim_end();
                let parsed_res = ValueFromStr::from_str(value);
                if let Ok(value) = parsed_res {
                    self.value = Some(value);
                }
            }
        }

        self
    }

    fn try_from_config_value(&mut self) -> &mut Self {
        if self.value.is_some() {
            return self;
        }

        if let Some((toml_table, ref key)) = self.config_value {
            let toml_value = toml_table.get(key);
            if let Some(toml_value) = toml_value {
                if let Some(toml_str) = toml_value.as_str() {
                    let value = toml_str;
                    let parsed_res = ValueFromStr::from_str(value);
                    if let Ok(value) = parsed_res {
                        self.value = Some(value);
                    }
                }
            }
        }

        self
    }

    fn try_from_default_value(&mut self) -> &mut Self {
        if self.value.is_some() {
            return self;
        }

        if self.default_value.is_some() {
            self.value = self.default_value.take();
        }

        self
    }

    pub fn build(&mut self) -> Result<T> {
        self.try_from_env();
        self.try_from_clap();
        self.try_from_clap_bool();
        self.try_from_clap_occurences();
        self.try_from_file_line();
        self.try_from_config_value();
        self.try_from_default_value();
        self.value
            .take()
            .ok_or_else(|| eyre!(format!("Missing value for config option: '{}'", self.key)))
    }
}

pub struct Builder<'clap> {
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
}

impl<'clap> Builder<'clap> {
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

        Builder {
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
        let simple_mode_domains =
            Builder::simple_mode_domains_as_records(self.build_simple_mode_domain_config_values());
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

    fn build_general_options(
        &self,
    ) -> Result<(
        UpdateInterval,
        SecretDigitalOceanToken,
        tracing::Level,
        bool,
        bool,
        bool,
    )> {
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

        Ok((
            update_interval,
            digital_ocean_token,
            log_level,
            dry_run,
            ipv4,
            ipv6,
        ))
    }

    pub fn build(&self) -> Result<Config> {
        let (update_interval, digital_ocean_token, log_level, dry_run, ipv4, ipv6) =
            self.build_general_options()?;
        let domains = self.build_domains()?;

        let config = Config {
            domains,
            update_interval,
            digital_ocean_token: Some(digital_ocean_token),
            log_level,
            dry_run,
            ipv4,
            ipv6,
        };
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::get_cli_command_definition;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_value_builder() {
        std::env::set_var(make_env_var_from_key("var1"), "some_val");
        let mut builder = ValueBuilder::<String>::new("var1");
        builder.with_env_var_name();
        let value = builder.build().unwrap();
        assert_eq!(value, "some_val");

        let arg_vec = vec!["my_prog", "--foo", "some_val"];
        let matches = clap::Command::new("test")
            .arg(clap::Arg::new("foo").short('f').long("foo"))
            .get_matches_from(arg_vec);
        let mut builder = ValueBuilder::<String>::new("foo");
        builder.with_clap(Some(&matches));
        let value = builder.build().unwrap();
        assert_eq!(value, "some_val");

        {
            use std::io::Write;
            let mut file = NamedTempFile::new().unwrap();
            writeln!(file, "some_val").unwrap();
            let temp_file_path = file.path();
            let mut builder = ValueBuilder::<String>::new("some_file");
            builder.with_single_line_from_file(temp_file_path.to_str().unwrap());
            let value = builder.build().unwrap();
            assert_eq!(value, "some_val");
        }

        let toml_value: toml::Value = toml::from_str(
            r#"
        some_field = 'some_val'
        "#,
        )
        .unwrap();
        let toml_map = toml_value.as_table().unwrap();
        let mut builder = ValueBuilder::<String>::new("some_field");
        builder.with_config_value(Some(toml_map));
        let value = builder.build().unwrap();
        assert_eq!(value, "some_val");

        let mut builder = ValueBuilder::<String>::new("default");
        builder.with_default("some_val".to_owned());
        let value = builder.build().unwrap();
        assert_eq!(value, "some_val");
    }

    #[test]
    fn verify_cli() {
        get_cli_command_definition().debug_assert()
    }
}
