use color_eyre::eyre::{eyre, Error};
use std::{fmt::Display, net::IpAddr};
use tracing::Level;

use crate::config::app_config::UpdateInterval;
use crate::token::SecretDigitalOceanToken;

pub mod api {
    use serde::Deserialize;
    #[derive(Deserialize, Debug)]
    pub struct DomainRecord {
        pub id: u64,
        #[serde(rename = "type")]
        pub record_type: String,
        pub name: String,
        // This contains the API response IP address.
        pub data: String,
    }

    #[derive(Deserialize, Debug)]
    pub struct DomainRecords {
        pub domain_records: Vec<DomainRecord>,
    }

    pub type DomainRecordCache = std::collections::HashMap<String, DomainRecords>;
    #[derive(Deserialize, Debug)]
    pub struct UpdateDomainRecordResponse {
        pub domain_record: DomainRecord,
    }
}

#[derive(Debug)]
pub struct DomainRecordToUpdate {
    pub domain_name: String,
    pub hostname_part: String,
    pub record_type: String,
}

impl DomainRecordToUpdate {
    pub fn new(domain_name: &str, hostname_part: &str, record_type: &str) -> Self {
        DomainRecordToUpdate {
            domain_name: domain_name.to_owned(),
            hostname_part: hostname_part.to_owned(),
            record_type: record_type.to_owned(),
        }
    }

    pub fn fqdn(&self) -> String {
        if self.hostname_part == "@" {
            self.domain_name.to_owned()
        } else {
            format!("{}.{}", self.hostname_part, self.domain_name)
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct IpAddrV4AndV6 {
    pub ipv4: Option<std::net::Ipv4Addr>,
    pub ipv6: Option<std::net::Ipv6Addr>,
}

#[derive(Debug, Copy, Clone)]
pub enum IpAddrKind {
    V4,
    V6,
}

impl Display for IpAddrV4AndV6 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ipv4: {:?} ipv6: {:?}", self.ipv4, self.ipv6)
    }
}

pub struct DisplayIpAddrV4AndV6Pretty<'a>(pub &'a IpAddrV4AndV6);

impl<'a> Display for DisplayIpAddrV4AndV6Pretty<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.has_both() {
            writeln!(f, "Public IPs are:")?;
            writeln!(f, "  ipv4: {}", self.0.to_ip_addr_from_ipv4())?;
            writeln!(f, "  ipv6: {}", self.0.to_ip_addr_from_ipv6())?;
        } else if self.0.has_any() {
            write!(f, "Public IP is: ")?;
            if self.0.has_ipv4() {
                writeln!(f, "{}", self.0.to_ip_addr_from_ipv4())?;
            }
            if self.0.has_ipv6() {
                writeln!(f, "{}", self.0.to_ip_addr_from_ipv6())?;
            }
        } else {
            writeln!(f, "No public IP")?;
        }
        Ok(())
    }
}

impl IpAddrV4AndV6 {
    pub fn has_none(&self) -> bool {
        self.ipv4.is_none() && self.ipv6.is_none()
    }

    pub fn has_any(&self) -> bool {
        self.ipv4.is_some() || self.ipv6.is_some()
    }

    pub fn has_both(&self) -> bool {
        self.ipv4.is_some() && self.ipv6.is_some()
    }

    pub fn has_ipv4(&self) -> bool {
        self.ipv4.is_some()
    }

    pub fn has_ipv6(&self) -> bool {
        self.ipv6.is_some()
    }

    pub fn to_ip_addr_from_ipv4(&self) -> IpAddr {
        IpAddr::V4(self.ipv4.expect("Does not contain a ipv4 address"))
    }

    pub fn to_ip_addr_from_ipv6(&self) -> IpAddr {
        IpAddr::V6(self.ipv6.expect("Does not contain a ipv4 address"))
    }

    pub fn to_ipv4_string(&self) -> Option<String> {
        self.ipv4.map(|ip| ip.to_string())
    }

    pub fn to_ipv6_string(&self) -> Option<String> {
        self.ipv6.map(|ip| ip.to_string())
    }

    pub fn to_ip_addr_from_any(&self) -> (IpAddr, IpAddrKind) {
        if self.has_ipv4() {
            (self.to_ip_addr_from_ipv4(), IpAddrKind::V4)
        } else {
            (self.to_ip_addr_from_ipv6(), IpAddrKind::V6)
        }
    }
}

impl From<IpAddr> for IpAddrV4AndV6 {
    fn from(value: IpAddr) -> Self {
        match value {
            IpAddr::V4(ip) => IpAddrV4AndV6 {
                ipv4: Some(ip),
                ..Default::default()
            },
            IpAddr::V6(ip) => IpAddrV4AndV6 {
                ipv6: Some(ip),
                ..Default::default()
            },
        }
    }
}

pub trait ValueFromStr: Sized {
    type Err;
    fn from_str(s: &str) -> Result<Self, Self::Err>;
}

// This doesn't work with the following error:
//
// conflicting implementations of trait `types::ValueFromStr` for type 'x'
// upstream crates may add a new impl of trait `std::str::FromStr` for type 'x' in future versions
//
// Apparently the recommended workaround is to use macros and be explicit
// about types.
// impl<T> ValueFromStr for T
// where
//     T: std::str::FromStr,
// {
//     type Err = T::Err;
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         s.parse::<T>()
//     }
// }

macro_rules! impl_value_from_str {
    ( $($t:ty),* ) => {
        $( impl ValueFromStr for $t {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self, Self::Err>
            {
                match s.parse::<$t>() {
                    Ok(val) => Ok(val),
                    Err(e) => Err(e.into()),
                }
            }
        })*
    }
}

impl_value_from_str! { String, bool, UpdateInterval, Level, u16 }

pub trait ValueFromBool: Sized {
    type Err;
    fn from_bool(b: bool) -> Result<Self, Self::Err>;
}

impl ValueFromBool for bool {
    type Err = Error;
    fn from_bool(b: bool) -> Result<Self, Self::Err> {
        Ok(b)
    }
}

macro_rules! impl_value_from_bool_as_error {
    ( $($t:ty),* ) => {
        $( impl ValueFromBool for $t {
            type Err = Error;
            fn from_bool(_b: bool) -> Result<Self, Self::Err>
            {
                 Err(eyre!("Can not convert from bool to target type"))
            }
        })*
    }
}

impl_value_from_bool_as_error! { String, UpdateInterval, Level, SecretDigitalOceanToken, u16 }
