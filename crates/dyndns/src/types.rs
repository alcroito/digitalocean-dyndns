use anyhow::Error;

pub mod api {
    use serde::Deserialize;
    #[derive(Deserialize, Debug)]
    pub struct DomainRecord {
        pub id: u64,
        #[serde(rename = "type")]
        pub record_type: String,
        pub name: String,
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
pub struct DomainRecordToUpdate<'d, 'h, 'r> {
    pub domain_name: &'d str,
    pub hostname_part: &'h str,
    pub record_type: &'r str,
}

impl<'d, 'h, 'r> DomainRecordToUpdate<'d, 'h, 'r> {
    pub fn new(domain_name: &'d str, hostname_part: &'h str, record_type: &'r str) -> Self {
        DomainRecordToUpdate {
            domain_name,
            hostname_part,
            record_type,
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

impl_value_from_str! { String, bool, crate::config::UpdateInterval, tracing::Level }
