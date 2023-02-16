use super::consts::*;
use crate::types::{ValueFromBool, ValueFromStr};
use clap::ArgMatches;
use color_eyre::eyre::{eyre, Result};

pub fn make_env_var_from_key(key: &str) -> String {
    format!("{}{}", ENV_VAR_PREFIX, key.to_ascii_uppercase())
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

impl<'clap, 'toml, T: ValueFromStr + ValueFromBool> ValueBuilder<'clap, 'toml, T> {
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
            // We only care about values that come from the command line, not default ones set by
            // clap. It's not possible to configure a clap argument to return None by default when
            // .action(clap::ArgAction::SetTrue/SetFalse) is used and no argument is specified
            // on the command line. So we only retrieve the value if it's a non-default one.
            if arg_matches.contains_id(option_name)
                && arg_matches
                    .value_source(option_name)
                    .expect("checked contains_id")
                    == clap::parser::ValueSource::CommandLine
            {
                let value = arg_matches.get_flag(option_name);
                let parsed_res = ValueFromBool::from_bool(value);
                self.value = parsed_res.ok();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::get_cli_command_definition;
    use tempfile::NamedTempFile;

    #[test]
    fn test_env_var() {
        // Happy path
        let key = "valid_env_var";
        std::env::set_var(make_env_var_from_key(key), "some_val");
        let mut builder = ValueBuilder::<String>::new(key);
        builder.with_env_var_name();
        let value = builder.build().unwrap();
        assert_eq!(value, "some_val");

        // Empty value
        let key = "empty_env_var";
        std::env::set_var(make_env_var_from_key(key), "");
        let mut builder = ValueBuilder::<String>::new(key);
        builder.with_env_var_name();
        let value = builder.build().unwrap();
        assert_eq!(value, "");

        // Env does not exist
        let key = "non_existent_env_var";
        let mut builder = ValueBuilder::<String>::new(key);
        builder.with_env_var_name();
        let value = builder.build();
        assert!(value.is_err());
    }

    #[test]
    fn test_clap_string_value() {
        let command = clap::Command::new("test").arg(clap::Arg::new("foo").short('f').long("foo"));

        // Happy path
        let arg_vec = vec!["my_prog", "--foo", "some_val"];
        let matches = command.clone().get_matches_from(arg_vec);
        let mut builder = ValueBuilder::<String>::new("foo");
        builder.with_clap(Some(&matches));
        let value = builder.build().unwrap();
        assert_eq!(value, "some_val");

        // Empty value
        let arg_vec = vec!["my_prog", "--foo", ""];
        let matches = command.clone().get_matches_from(arg_vec);
        let mut builder = ValueBuilder::<String>::new("foo");
        builder.with_clap(Some(&matches));
        let value = builder.build().unwrap();
        assert_eq!(value, "");

        // Key not given
        let arg_vec = vec!["my_prog"];
        let matches = command.get_matches_from(arg_vec);
        let mut builder = ValueBuilder::<String>::new("foo");
        builder.with_clap(Some(&matches));
        let value = builder.build();
        assert!(value.is_err());

        // Value not given
        let command = clap::Command::new("test")
            .arg(clap::Arg::new("foo").short('f').long("foo").num_args(0..=1));
        let arg_vec = vec!["my_prog", "--foo"];
        let matches = command.get_matches_from(arg_vec);
        let mut builder = ValueBuilder::<String>::new("foo");
        builder.with_clap(Some(&matches));
        let value = builder.build();
        assert!(value.is_err());
    }

    #[test]
    fn test_clap_bool_value() {
        let command = clap::Command::new("test").arg(
            clap::Arg::new("foo")
                .short('f')
                .action(clap::ArgAction::SetTrue),
        );

        // clap option is false by default when unset, option set, thus result is Some(true)
        let arg_vec = vec!["my_prog", "-f"];
        let matches = command.clone().get_matches_from(arg_vec);
        let mut builder = ValueBuilder::<bool>::new("foo");
        builder.with_clap_bool(Some(&matches));
        let value = builder.build().unwrap();
        assert!(value);

        // clap option is false by default when unset, option unset, thus result is None
        let arg_vec = vec!["my_prog"];
        let matches = command.get_matches_from(arg_vec);
        let mut builder = ValueBuilder::<bool>::new("foo");
        builder.with_clap_bool(Some(&matches));
        let value = builder.build();
        assert!(value.is_err());

        let command = clap::Command::new("test").arg(
            clap::Arg::new("foo")
                .short('f')
                .action(clap::ArgAction::SetFalse),
        );

        // clap option is true by default when unset, option set, thus result is Some(false)
        let arg_vec = vec!["my_prog", "-f"];
        let matches = command.clone().get_matches_from(arg_vec);
        let mut builder = ValueBuilder::<bool>::new("foo");
        builder.with_clap_bool(Some(&matches));
        let value = builder.build().unwrap();
        assert!(!value);

        // clap option is true by default when unset, option unset, thus result is None
        let arg_vec = vec!["my_prog"];
        let matches = command.get_matches_from(arg_vec);
        let mut builder = ValueBuilder::<bool>::new("foo");
        builder.with_clap_bool(Some(&matches));
        let value = builder.build();
        assert!(value.is_err());
    }

    #[test]
    fn test_clap_occurrences_value() {
        let command = clap::Command::new("test").arg(
            clap::Arg::new("v")
                .short('v')
                .action(clap::ArgAction::Count),
        );

        // Happy path 2 values
        let arg_vec = vec!["my_prog", "-vv"];
        let matches = command.clone().get_matches_from(arg_vec);
        let mut builder = ValueBuilder::<String>::new("v");
        builder.with_clap_occurences(Some(&matches), "v", Box::new(|v| Some(v.to_string())));
        let value = builder.build().unwrap();
        assert_eq!(value, "2");

        // Happy path 1 value
        let arg_vec = vec!["my_prog", "-v"];
        let matches = command.clone().get_matches_from(arg_vec);
        let mut builder = ValueBuilder::<String>::new("v");
        builder.with_clap_occurences(Some(&matches), "v", Box::new(|v| Some(v.to_string())));
        let value = builder.build().unwrap();
        assert_eq!(value, "1");

        // Happy path 0 values
        let arg_vec = vec!["my_prog"];
        let matches = command.get_matches_from(arg_vec);
        let mut builder = ValueBuilder::<String>::new("v");
        builder.with_clap_occurences(Some(&matches), "v", Box::new(|v| Some(v.to_string())));
        let value = builder.build().unwrap();
        assert_eq!(value, "0");
    }

    #[test]
    fn test_file_line() {
        use std::io::Write;
        // Happy path
        {
            let mut file = NamedTempFile::new().unwrap();
            writeln!(file, "some_val").unwrap();
            let temp_file_path = file.path();
            let mut builder = ValueBuilder::<String>::new("some_file");
            builder.with_single_line_from_file(temp_file_path.to_str().unwrap());
            let value = builder.build().unwrap();
            assert_eq!(value, "some_val");
        }

        // FIXME: Empty file is valid, but should be an error
        {
            let file = NamedTempFile::new().unwrap();
            let temp_file_path = file.path();
            let mut builder = ValueBuilder::<String>::new("some_file");
            builder.with_single_line_from_file(temp_file_path.to_str().unwrap());
            let value = builder.build().unwrap();
            assert_eq!(value, "");
        }

        // Missing file
        {
            let temp_file_path = "/definitely_should_not_exist";
            let mut builder = ValueBuilder::<String>::new("some_file");
            builder.with_single_line_from_file(temp_file_path);
            let value = builder.build();
            assert!(value.is_err());
        }
    }

    #[test]
    fn test_toml_value() {
        let toml_value: toml::Value = toml::from_str(
            r#"
        some_field = 'some_val'
        empty_field = ''
        "#,
        )
        .unwrap();

        // Happy path
        let toml_map = toml_value.as_table().unwrap();
        let mut builder = ValueBuilder::<String>::new("some_field");
        builder.with_config_value(Some(toml_map));
        let value = builder.build().unwrap();
        assert_eq!(value, "some_val");

        // Empty field
        let toml_map = toml_value.as_table().unwrap();
        let mut builder = ValueBuilder::<String>::new("empty_field");
        builder.with_config_value(Some(toml_map));
        let value = builder.build().unwrap();
        assert_eq!(value, "");

        // Missing key
        let toml_map = toml_value.as_table().unwrap();
        let mut builder = ValueBuilder::<String>::new("missing_field");
        builder.with_config_value(Some(toml_map));
        let value = builder.build();
        assert!(value.is_err());
    }

    #[test]
    fn test_default_value() {
        // Happy path
        let mut builder = ValueBuilder::<String>::new("foo");
        builder.with_default("some_val".to_owned());
        let value = builder.build().unwrap();
        assert_eq!(value, "some_val");

        // Empty string
        let mut builder = ValueBuilder::<String>::new("foo");
        builder.with_default("".to_owned());
        let value = builder.build().unwrap();
        assert_eq!(value, "");

        // Missing key
        let mut builder = ValueBuilder::<String>::new("foo");
        let value = builder.build();
        assert!(value.is_err());
    }

    #[test]
    fn verify_cli() {
        get_cli_command_definition().debug_assert()
    }
}
