use anyhow::{anyhow, Context, Result};
use log::info;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use trust_dns_resolver::config::{NameServerConfigGroup, ResolverConfig, ResolverOpts};
use trust_dns_resolver::Resolver;

/// IP addresses for OpenDNS Public DNS
/// https://en.wikipedia.org/wiki/OpenDNS
const OPEN_DNS_IPS: &[IpAddr] = &[
    IpAddr::V4(Ipv4Addr::new(208, 67, 222, 222)),
    IpAddr::V4(Ipv4Addr::new(208, 67, 220, 220)),
    IpAddr::V6(Ipv6Addr::new(0x2620, 0x119, 0x35, 0, 0, 0, 0, 0x35)),
    IpAddr::V6(Ipv6Addr::new(0x2620, 0x119, 0x53, 0, 0, 0, 0, 0x53)),
];

pub trait PublicIpFetcher {
    fn fetch_public_ip(&self) -> Result<IpAddr>;
}

#[derive(Default)]
pub struct DnsIpFetcher {}

impl PublicIpFetcher for DnsIpFetcher {
    /// Fetch public IP of current machine by querying the OpenDNS myip resolver
    /// See https://unix.stackexchange.com/questions/22615/how-can-i-get-my-external-ip-address-in-a-shell-script/81699#81699
    fn fetch_public_ip(&self) -> Result<IpAddr> {
        info!("Fetching public IP using OpenDNS");
        let hostname_to_lookup = "myip.opendns.com.";
        let resolver_config = ResolverConfig::from_parts(
            None,
            vec![],
            NameServerConfigGroup::from_ips_clear(OPEN_DNS_IPS, 53, true),
        );
        let resolver = Resolver::new(resolver_config, ResolverOpts::default())?;
        let response = resolver
            .lookup_ip(hostname_to_lookup)
            .context("Failed to resolve public IP address")?;
        let address = response.iter().next().ok_or_else(|| {
            anyhow!("Failed to find public IP address: no addresses returned from DNS resolution")
        })?;
        info!("Public IP is: {}", address);
        Ok(address)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[derive(Default)]
    pub struct MockIpFetcher {}

    impl PublicIpFetcher for MockIpFetcher {
        fn fetch_public_ip(&self) -> Result<IpAddr> {
            Ok(IpAddr::V4(Ipv4Addr::new(85, 212, 89, 12)))
        }
    }
}
