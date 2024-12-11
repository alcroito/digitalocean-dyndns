use color_eyre::eyre::{bail, Result, WrapErr};
use hickory_resolver::config::{
    LookupIpStrategy, NameServerConfigGroup, ResolverConfig, ResolverOpts,
};
use hickory_resolver::Resolver;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use tracing::info;

use crate::types::{DisplayIpAddrV4AndV6Pretty, IpAddrV4AndV6};

/// IP addresses for OpenDNS Public DNS
/// [https://en.wikipedia.org/wiki/OpenDNS](https://en.wikipedia.org/wiki/OpenDNS)
const OPEN_DNS_IPS: &[IpAddr] = &[
    IpAddr::V4(Ipv4Addr::new(208, 67, 222, 222)),
    IpAddr::V4(Ipv4Addr::new(208, 67, 220, 220)),
    IpAddr::V6(Ipv6Addr::new(0x2620, 0x119, 0x35, 0, 0, 0, 0, 0x35)),
    IpAddr::V6(Ipv6Addr::new(0x2620, 0x119, 0x53, 0, 0, 0, 0, 0x53)),
];

pub trait PublicIpFetcher {
    fn fetch_public_ips(&self, lookup_ipv4: bool, lookup_ipv6: bool) -> Result<IpAddrV4AndV6>;
}

#[derive(Default)]
pub struct DnsIpFetcher {}

impl PublicIpFetcher for DnsIpFetcher {
    /// Fetch public IP of current machine by querying the OpenDNS myip resolver
    /// See
    /// [Stack Overflow](https://unix.stackexchange.com/questions/22615/how-can-i-get-my-external-ip-address-in-a-shell-script/81699#81699)
    fn fetch_public_ips(&self, lookup_ipv4: bool, lookup_ipv6: bool) -> Result<IpAddrV4AndV6> {
        info!("Fetching public IP using OpenDNS");
        let hostname_to_lookup = "myip.opendns.com.";
        let resolver_config = ResolverConfig::from_parts(
            None,
            vec![],
            NameServerConfigGroup::from_ips_clear(OPEN_DNS_IPS, 53, true),
        );

        let mut resolver_options = ResolverOpts::default();
        resolver_options.use_hosts_file = false;

        resolver_options.ip_strategy = match (lookup_ipv4, lookup_ipv6) {
            (true, true) => LookupIpStrategy::Ipv4AndIpv6,
            (true, false) => LookupIpStrategy::Ipv4Only,
            (false, true) => LookupIpStrategy::Ipv6Only,
            (false, false) => unreachable!(),
        };

        resolver_options.attempts = 1;
        resolver_options.num_concurrent_reqs = 1;

        let resolver_options = resolver_options;
        let resolver = Resolver::new(resolver_config, resolver_options)?;

        let response = resolver
            .lookup_ip(hostname_to_lookup)
            .wrap_err("Failed to resolve public IP address")?;

        let address =
            response
                .iter()
                .fold(IpAddrV4AndV6::default(), |mut maybe_ip, response_ip| {
                    match response_ip {
                        IpAddr::V4(ip) if maybe_ip.ipv4.is_none() => maybe_ip.ipv4 = Some(ip),
                        IpAddr::V6(ip) if maybe_ip.ipv6.is_none() => maybe_ip.ipv6 = Some(ip),
                        _ => (),
                    }
                    maybe_ip
                });

        if address.has_none() {
            bail!("Failed to find public IP address: no addresses returned from DNS resolution");
        }
        info!("{}", DisplayIpAddrV4AndV6Pretty(&address));
        Ok(address)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[derive(Default)]
    pub struct MockIpFetcher {}

    impl PublicIpFetcher for MockIpFetcher {
        fn fetch_public_ips(&self, _: bool, _: bool) -> Result<IpAddrV4AndV6> {
            Ok(IpAddr::V4(Ipv4Addr::new(85, 212, 89, 12)).into())
        }
    }
}
