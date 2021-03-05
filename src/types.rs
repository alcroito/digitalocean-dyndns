use anyhow::Error;
use serde::Deserialize;
#[derive(Deserialize, Debug)]
pub struct DomainRecord {
    pub id: u64,
    #[serde(rename = "type")]
    pub domain_type: String,
    pub name: String,
    pub data: String,
}

#[derive(Deserialize, Debug)]
pub struct DomainRecords {
    pub domain_records: Vec<DomainRecord>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateDomainRecordResponse {
    pub domain_record: DomainRecord,
}

pub struct SubdomainFilter {
    domain_root: String,
    subdomain: String,
}

impl SubdomainFilter {
    pub fn new(domain_root: &str, subdomain: &str) -> Self {
        SubdomainFilter {
            domain_root: domain_root.to_owned(),
            subdomain: subdomain.to_owned(),
        }
    }
}

pub enum DomainFilter {
    Root(String),
    Subdomain(SubdomainFilter),
}

impl DomainFilter {
    pub fn new(domain_root: &str, hostname_part: &str) -> Self {
        if hostname_part == "@" {
            DomainFilter::Root(domain_root.to_owned())
        } else {
            DomainFilter::Subdomain(SubdomainFilter::new(domain_root, hostname_part))
        }
    }

    pub fn fqdn(&self) -> String {
        match self {
            DomainFilter::Root(domain_root) => domain_root.to_string(),
            DomainFilter::Subdomain(filter) => {
                format!("{}.{}", filter.subdomain, filter.domain_root)
            }
        }
    }

    pub fn record_type(&self) -> &str {
        match self {
            DomainFilter::Root(_) => "A",
            DomainFilter::Subdomain(_) => "A",
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

impl_value_from_str! { String, bool, crate::config::UpdateInterval, log::LevelFilter }
