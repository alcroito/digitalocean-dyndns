use clap::{crate_version, App, Arg, ArgMatches};

use crate::config_consts::*;

pub fn get_clap_matches() -> ArgMatches<'static> {
    let matches = App::new("DigitalOcean dynamic dns updater")
        .version(crate_version!())
        .about("Updates a DigitalOcean domain record to point to the current machine's public IP")
        .setting(clap::AppSettings::NextLineHelp)
        .usage(
            "\
    do_dyndns [FLAGS] [OPTIONS]
    do_dyndns -c <CONFIG_PATH> -d <DOMAIN> -s <SUBDOMAIN> -t <TOKEN> -p <TOKEN_PATH>
    do_dyndns -d <DOMAIN> -r -t <TOKEN>
    do_dyndns -c /config/ddns.toml -t <TOKEN>
    do_dyndns -vvv -d foo.net -s home -i '10 mins' -p <TOKEN_PATH>
",
        )
        .after_help("Instead of using command line options you can also set environment variables.")
        .arg(
            Arg::with_name(CONFIG_KEY)
                .short("c")
                .long(CONFIG_KEY)
                .value_name("FILE")
                .help(
                    "\
Path to TOML config file.
Default config path when none specified: '$PWD/config/do_ddns.toml'
Env var: DO_DYNDNS_CONFIG=/config/do_ddns.toml",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name(LOG_LEVEL_VERBOSITY_SHORT)
                .short(LOG_LEVEL_VERBOSITY_SHORT)
                .multiple(true)
                .help(
                    "\
Increases the level of verbosity. Repeat for more verbosity.
Env var: DO_DYNDNS_LOG_LEVEL=info [error|warn|info|debug|trace]
",
                ),
        )
        .arg(
            Arg::with_name(DOMAIN_ROOT)
                .short("d")
                .long("domain-root")
                .value_name("DOMAIN")
                .help(
                    "\
The domain root for which the domain record will be changed.
Example: 'foo.net'
Env var: DO_DYNDNS_DOMAIN_ROOT=foo.net",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name(SUBDOMAIN_TO_UPDATE)
                .short("s")
                .long("subdomain-to-update")
                .value_name("SUBDOMAIN")
                .help(
                    "\
The subdomain for which the public IP will be updated.
Example: 'home'
Env var: DO_DYNDNS_SUBDOMAIN_TO_UPDATE=home",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name(UPDATE_DOMAIN_ROOT)
                .short("r")
                .long("update-domain-root")
                .help(
                    "\
If true, the provided domain root 'A' record will be updated (instead of a subdomain).
Env var: DO_DYNDNS_UPDATE_DOMAIN_ROOT=true",
                )
                .conflicts_with(SUBDOMAIN_TO_UPDATE),
        )
        .arg(
            Arg::with_name(DIGITAL_OCEAN_TOKEN)
                .short("t")
                .long("token")
                .value_name("TOKEN")
                .help(
                    "\
The digital ocean access token.
Example: 'abcdefghijklmnopqrstuvwxyz'
Env var: DO_DYNDNS_DIGITAL_OCEAN_TOKEN=abcdefghijklmnopqrstuvwxyz",
                )
                .takes_value(true),
        )
        .arg(
            Arg::with_name(DIGITAL_OCEAN_TOKEN_PATH)
                .short("p")
                .long("token-file-path")
                .value_name("FILE_PATH")
                .help(
                    "\
Path to file containing the digital ocean token on its first line.
Example: '/config/secret_token.txt'",
                )
                .takes_value(true)
                .conflicts_with(DIGITAL_OCEAN_TOKEN),
        )
        .arg(
            Arg::with_name(UPDATE_INTERVAL)
                .short("i")
                .long("update-interval")
                .value_name("INTERVAL")
                .help(
                    "\
How often should the domain be updated.
Default is every 10 minutes.
Uses rust's humantime format.
Example: '15 mins 30 secs'
Env var: DO_DYNDNS_UPDATE_INTERVAL=2hours 30mins",
                )
                .takes_value(true),
        )
        .arg(Arg::with_name(DRY_RUN).short("n").long("dry-run").help(
            "\
Show what would have been updated.
Env var: DO_DYNDNS_DRY_RUN=true",
        ))
        .get_matches();
    matches
}
