use color_eyre::eyre::{bail, Result, WrapErr};
use hickory_resolver::config::{
    LookupIpStrategy, NameServerConfigGroup, ResolveHosts, ResolverConfig, ResolverOpts,
};
use hickory_resolver::{name_server::TokioConnectionProvider, Resolver};
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
        info!(
            "Fetching public IP using OpenDNS, IPv4: {}, IPv6: {}",
            lookup_ipv4, lookup_ipv6
        );
        let hostname_to_lookup = "myip.opendns.com.";

        let mut result = IpAddrV4AndV6::default();

        if lookup_ipv4 {
            let ipv4_dns_ips = OPEN_DNS_IPS.get(..2).expect("No IPv4 addresses");
            let resolver_config = ResolverConfig::from_parts(
                None,
                vec![],
                NameServerConfigGroup::from_ips_clear(ipv4_dns_ips, 53, true),
            );

            let mut resolver_options = ResolverOpts::default();
            resolver_options.use_hosts_file = ResolveHosts::Never;
            resolver_options.ip_strategy = LookupIpStrategy::Ipv4Only;
            resolver_options.attempts = 1;
            resolver_options.num_concurrent_reqs = 1;

            let mut builder =
                Resolver::builder_with_config(resolver_config, TokioConnectionProvider::default());
            *builder.options_mut() = resolver_options;
            let resolver = builder.build();

            let response = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(resolver.lookup_ip(hostname_to_lookup))
            })
            .wrap_err("Failed to resolve IPv4 public IP address")?;

            for response_ip in response.iter() {
                if let IpAddr::V4(ip) = response_ip {
                    result.ipv4 = Some(ip);
                    break;
                }
            }
        }

        if lookup_ipv6 {
            let ipv6_dns_ips = OPEN_DNS_IPS.get(2..).expect("No IPv6 addresses");
            let resolver_config = ResolverConfig::from_parts(
                None,
                vec![],
                NameServerConfigGroup::from_ips_clear(ipv6_dns_ips, 53, true),
            );

            let mut resolver_options = ResolverOpts::default();
            resolver_options.use_hosts_file = ResolveHosts::Never;
            resolver_options.ip_strategy = LookupIpStrategy::Ipv6Only;
            resolver_options.attempts = 1;
            resolver_options.num_concurrent_reqs = 1;

            let mut builder =
                Resolver::builder_with_config(resolver_config, TokioConnectionProvider::default());
            *builder.options_mut() = resolver_options;
            let resolver = builder.build();

            let response = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(resolver.lookup_ip(hostname_to_lookup))
            })
            .wrap_err("Failed to resolve IPv6 public IP address")?;

            for response_ip in response.iter() {
                if let IpAddr::V6(ip) = response_ip {
                    result.ipv6 = Some(ip);
                    break;
                }
            }
        }

        if result.has_none() {
            bail!("Failed to find public IP address: no addresses returned from DNS resolution");
        }
        info!("{}", DisplayIpAddrV4AndV6Pretty(&result));
        Ok(result)
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
