use clap::{crate_version, ArgMatches, Args, Command};
use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::{config::app_config::UpdateInterval, token::SecretDigitalOceanToken};

pub fn get_cli_args() -> ArgMatches {
    get_cli_command_definition().get_matches()
}

#[skip_serializing_none]
#[derive(Args, Debug, Serialize)]
pub struct CommonArgs {
    /// Path to TOML config file.
    ///
    /// Default config path when none specified: `$PWD/config/do_ddns.toml`
    /// Env var: `DO_DYNDNS_CONFIG=/config/do_ddns.toml`,
    #[arg(short = 'c', long = "config", id = "config")]
    pub config_file_path: Option<String>,

    /// Increases the level of verbosity. Repeat for more verbosity.
    ///
    /// Env var: `DO_DYNDNS_LOG_LEVEL=info` [error|warn|info|debug|trace]
    #[arg(short = 'v', action = clap::ArgAction::Count, id = "v")]
    pub log_level: Option<u8>,

    /// The domain root for which the domain record will be changed.
    ///
    /// Example: 'foo.net'
    /// Env var: `DO_DYNDNS_DOMAIN_ROOT=foo.net`
    #[arg(short = 'd', long)]
    pub domain_root: Option<String>,

    /// The subdomain for which the public IP will be updated.
    ///
    /// Example: 'home'
    /// Env var: `DO_DYNDNS_SUBDOMAIN_TO_UPDATE=home`
    #[arg(short = 's', long)]
    pub subdomain_to_update: Option<String>,

    /// If true, the provided domain root 'A' record will be updated (instead of a subdomain).
    ///
    /// Env var: `DO_DYNDNS_UPDATE_DOMAIN_ROOT=true`
    #[arg(short = 'r', long, conflicts_with = "subdomain_to_update", default_missing_value = "true", num_args = 0..=1)]
    pub update_domain_root: Option<bool>,

    /// The digital ocean access token.
    ///
    /// Example: 'abcdefghijklmnopqrstuvwxyz'
    /// Env var: `DO_DYNDNS_DIGITAL_OCEAN_TOKEN=abcdefghijklmnopqrstuvwxyz`
    #[arg(short = 't', long, value_parser = crate::token::parse_secret_token)]
    pub digital_ocean_token: Option<SecretDigitalOceanToken>,

    /// Path to file containing the digital ocean token on its first line.
    ///
    /// Example: `/config/secret_token.txt`
    #[arg(
        short = 'p',
        long = "token-file-path",
        conflicts_with = "digital_ocean_token",
        id = "token_file_path"
    )]
    pub digital_ocean_token_path: Option<String>,

    /// How often should the domain be updated.
    ///
    /// Default is every 10 minutes.
    /// Uses rust's humantime format.
    /// Example: '15 mins 30 secs'
    /// Env var: `DO_DYNDNS_UPDATE_INTERVAL=2hours 30mins`
    #[arg(short = 'i', long)]
    pub update_interval: Option<UpdateInterval>,

    /// Show what would have been updated.
    ///
    /// Env var: `DO_DYNDNS_DRY_RUN=true`
    #[arg(short = 'n', long, default_missing_value = "true", num_args = 0..=1)]
    pub dry_run: Option<bool>,

    /// Enable ipv6 support (disabled by default).
    ///
    /// Env var: `DO_DYNDNS_IPV6_SUPPORT=true`
    // num_args + default_missing_value emulates a flag action::SetTrue, which
    // preserves None when nothing is passed
    #[arg(long = "enable-ipv6", id = "ipv6", default_missing_value = "true", num_args = 0..=1)]
    #[serde(rename = "ipv6")]
    pub ipv6_support: Option<bool>,

    /// Output build info like git commit sha, rustc version, etc
    #[arg(long = "build-info")]
    pub build_info: bool,
}

#[skip_serializing_none]
#[derive(Args, Debug, Serialize)]
pub struct ConditionalArgs {
    /// Enable collection of statistics (how often does the public IP change).
    ///
    /// Env var: `DO_DYNDNS_COLLECT_STATS=true`
    #[arg(long, default_missing_value = "true", num_args = 0..=1)]
    #[cfg_attr(not(feature = "stats"), arg(hide = true))]
    pub collect_stats: Option<bool>,

    /// File path where a sqlite database with statistics will be stored.
    ///
    /// Env var: `DO_DYNDNS_DATABASE_PATH=/tmp/dyndns_stats_db.sqlite`
    #[arg(long = "database-path", id = "database_path")]
    #[cfg_attr(not(feature = "stats"), arg(hide = true))]
    #[serde(rename = "database_path")]
    pub db_path: Option<std::path::PathBuf>,

    /// Enable web server to visualize collected statistics.
    ///
    /// Env var: `DO_DYNDNS_ENABLE_WEB=true`
    #[arg(long, default_missing_value = "true", num_args = 0..=1)]
    #[cfg_attr(not(feature = "web"), arg(hide = true))]
    pub enable_web: Option<bool>,

    /// An IP address or host name where to serve HTTP pages on.
    ///
    /// Env var: `DO_DYNDNS_LISTEN_HOSTNAME=192.168.0.1`
    #[arg(long)]
    #[cfg_attr(not(feature = "web"), arg(hide = true))]
    pub listen_hostname: Option<String>,

    /// Port numbere where to serve HTTP pages on.
    ///
    /// Env var: `DO_DYNDNS_LISTEN_PORT=8080`
    #[arg(long)]
    #[cfg_attr(not(feature = "web"), arg(hide = true))]
    pub listen_port: Option<u16>,
}

#[skip_serializing_none]
#[derive(Args, Debug, Serialize)]
pub struct ClapAllArgs {
    #[command(flatten)]
    #[serde(flatten)]
    common_args: CommonArgs,

    #[command(flatten)]
    #[serde(flatten)]
    conditional_args: ConditionalArgs,
}

impl ClapAllArgs {
    pub fn parse_and_process(clap_matches: &ArgMatches) -> Result<Self, color_eyre::eyre::Error> {
        use clap::FromArgMatches;
        let mut args = Self::from_arg_matches(clap_matches)?;
        // clap doesn't support generic mapping of argument values when using ArgAction::Count
        // So we manually reset the log level to None if count was 0 (aka none was specified).
        // This ensures the value is not serialized and used the by the configuration merging.
        if let Some(0) = args.common_args.log_level {
            args.common_args.log_level = None;
        }
        Ok(args)
    }
}

fn get_cli_command_definition_base() -> Command {
    Command::new("DigitalOcean dynamic dns updater")
        .version(crate_version!())
        .about("Updates DigitalOcean domain records to point to the current machine's public IP")
        .next_line_help(true)
        .override_usage(
            "\
    Simple config mode:
    do_dyndns [FLAGS] [OPTIONS]
    do_dyndns -c <CONFIG_PATH> -d <DOMAIN> -s <SUBDOMAIN> -t <TOKEN> -p <TOKEN_PATH>
    do_dyndns -d <DOMAIN> -r -t <TOKEN>
    do_dyndns -c /config/ddns.toml -t <TOKEN>
    do_dyndns -vvv -d foo.net -s home -i '10 mins' -p <TOKEN_PATH>

    Advanced config mode:
    do_dyndns -c /config/ddns.toml -t <TOKEN>
",
        )
        .after_help(
            "\
In simple config mode you can specify only one single domain record to update.
The domain record details can be provided either via command line options,
environment variables, or the config file.

In advanced config mode you can specify multiple domains and records to update.
The details can only be provided via the config file.

Below is a sample config file which updates multiple domains:
update_interval = \"10 minutes\"
digital_ocean_token = \"abcd\"
log_level = \"info\"

[[domains]]
name = \"mysite.com\"

# Updates home.mysite.com
[[domains.records]]
type = \"A\"
name = \"home\"

# Updates home-backup.mysite.com
[[domains.records]]
type = \"A\"
name = \"home-backup\"

[[domains]]
name = \"mysecondsite.com\"

# Updates mysecondsite.com
[[domains.records]]
type = \"A\"
name = \"@\"

# Updates crib.mysecondsite.com
[[domains.records]]
type = \"A\"
name = \"crib\"
",
        )
}

pub fn get_cli_command_definition() -> Command {
    let mut command = get_cli_command_definition_base();

    command = CommonArgs::augment_args(command);
    command = ConditionalArgs::augment_args(command);

    command
}
