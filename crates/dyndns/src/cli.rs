use clap::{crate_version, Arg, ArgMatches, Command};

use crate::config::consts::*;

pub fn get_cli_args() -> ArgMatches {
    get_cli_command_definition().get_matches()
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
        .arg(
            Arg::new(CONFIG_KEY)
                .short('c')
                .long(CONFIG_KEY)
                .value_name("FILE")
                .help(
                    "\
Path to TOML config file.
Default config path when none specified: '$PWD/config/do_ddns.toml'
Env var: DO_DYNDNS_CONFIG=/config/do_ddns.toml",
                ),
        )
        .arg(
            Arg::new(LOG_LEVEL_VERBOSITY_SHORT)
                .short(LOG_LEVEL_VERBOSITY_SHORT_CHAR)
                .action(clap::ArgAction::Count)
                .help(
                    "\
Increases the level of verbosity. Repeat for more verbosity.
Env var: DO_DYNDNS_LOG_LEVEL=info [error|warn|info|debug|trace]
",
                ),
        )
        .arg(
            Arg::new(DOMAIN_ROOT)
                .short('d')
                .long("domain-root")
                .value_name("DOMAIN")
                .help(
                    "\
The domain root for which the domain record will be changed.
Example: 'foo.net'
Env var: DO_DYNDNS_DOMAIN_ROOT=foo.net",
                ),
        )
        .arg(
            Arg::new(SUBDOMAIN_TO_UPDATE)
                .short('s')
                .long("subdomain-to-update")
                .value_name("SUBDOMAIN")
                .help(
                    "\
The subdomain for which the public IP will be updated.
Example: 'home'
Env var: DO_DYNDNS_SUBDOMAIN_TO_UPDATE=home",
                ),
        )
        .arg(
            Arg::new(UPDATE_DOMAIN_ROOT)
                .short('r')
                .long("update-domain-root")
                .help(
                    "\
If true, the provided domain root 'A' record will be updated (instead of a subdomain).
Env var: DO_DYNDNS_UPDATE_DOMAIN_ROOT=true",
                )
                .action(clap::ArgAction::SetTrue)
                .conflicts_with(SUBDOMAIN_TO_UPDATE),
        )
        .arg(
            Arg::new(DIGITAL_OCEAN_TOKEN)
                .short('t')
                .long("token")
                .value_name("TOKEN")
                .help(
                    "\
The digital ocean access token.
Example: 'abcdefghijklmnopqrstuvwxyz'
Env var: DO_DYNDNS_DIGITAL_OCEAN_TOKEN=abcdefghijklmnopqrstuvwxyz",
                ),
        )
        .arg(
            Arg::new(DIGITAL_OCEAN_TOKEN_PATH)
                .short('p')
                .long("token-file-path")
                .value_name("FILE_PATH")
                .help(
                    "\
Path to file containing the digital ocean token on its first line.
Example: '/config/secret_token.txt'",
                )
                .conflicts_with(DIGITAL_OCEAN_TOKEN),
        )
        .arg(
            Arg::new(UPDATE_INTERVAL)
                .short('i')
                .long("update-interval")
                .value_name("INTERVAL")
                .help(
                    "\
How often should the domain be updated.
Default is every 10 minutes.
Uses rust's humantime format.
Example: '15 mins 30 secs'
Env var: DO_DYNDNS_UPDATE_INTERVAL=2hours 30mins",
                ),
        )
        .arg(
            Arg::new(DRY_RUN)
                .short('n')
                .long("dry-run")
                .action(clap::ArgAction::SetTrue)
                .help(
                    "\
Show what would have been updated.
Env var: DO_DYNDNS_DRY_RUN=true",
                ),
        )
        .arg(
            Arg::new(IPV6_SUPPORT)
                .long("enable-ipv6")
                .action(clap::ArgAction::SetTrue)
                .help(
                    "\
Enable ipv6 support (disabled by default).
Env var: DO_DYNDNS_IPV6_SUPPORT=true",
                ),
        )
        .arg(
            Arg::new(BUILD_INFO)
                .long("build-info")
                .help(
                    "\
Output build info like git commit sha, rustc version, etc",
                )
                .action(clap::ArgAction::SetTrue),
        )
}

pub fn get_cli_command_definition() -> Command {
    let mut command = get_cli_command_definition_base();

    // Don't show stats related options when building with the feature disabled.
    let mut arg = Arg::new(COLLECT_STATS)
        .long("collect-stats")
        .action(clap::ArgAction::SetTrue)
        .help(
            "\
Enable collection of statistics (how often does the public IP change).
Env var: DO_DYNDNS_COLLECT_STATS=true",
        );
    if cfg!(not(feature = "stats")) {
        arg = arg.hide(true);
    }
    command = command.arg(arg);

    let mut arg = Arg::new(DB_PATH).long("database-path").help(
        "\
File path where a sqlite database with statistics will be stored.
Env var: DO_DYNDNS_DATABASE_PATH=/tmp/dyndns_stats_db.sqlite",
    );

    if cfg!(not(feature = "stats")) {
        arg = arg.hide(true);
    }
    command = command.arg(arg);

    // Don't show web related options when building with the feature disabled.
    let mut arg = Arg::new(ENABLE_WEB)
        .long("enable-web")
        .action(clap::ArgAction::SetTrue)
        .help(
            "\
Enable web server to visualize collected statistics.
Env var: DO_DYNDNS_ENABLE_WEB=true",
        );
    if cfg!(not(feature = "web")) {
        arg = arg.hide(true);
    }
    command = command.arg(arg);

    let mut arg = Arg::new(LISTEN_HOSTNAME).long("listen-hostname").help(
        "\
An IP address or host name where to serve HTTP pages on.
Env var: DO_DYNDNS_LISTEN_HOSTNAME=192.168.0.1",
    );
    if cfg!(not(feature = "web")) {
        arg = arg.hide(true);
    }
    command = command.arg(arg);

    let mut arg = Arg::new(LISTEN_PORT).long("listen-port").help(
        "\
Port numbere where to serve HTTP pages on.
Env var: DO_DYNDNS_LISTEN_PORT=8080",
    );
    if cfg!(not(feature = "web")) {
        arg = arg.hide(true);
    }
    command = command.arg(arg);

    command
}
