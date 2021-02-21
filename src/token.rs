use crate::types::ValueFromStr;
use anyhow::Error;
use secrecy::Secret;

#[derive(Clone)]
pub struct DigitalOceanToken(String);

pub type SecretDigitalOceanToken = Secret<DigitalOceanToken>;

impl DigitalOceanToken {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl secrecy::Zeroize for DigitalOceanToken {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl secrecy::CloneableSecret for DigitalOceanToken {}
impl secrecy::DebugSecret for DigitalOceanToken {}

impl std::str::FromStr for DigitalOceanToken {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(DigitalOceanToken(s.to_owned()))
    }
}

impl ValueFromStr for Secret<DigitalOceanToken> {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Secret::new(s.parse::<DigitalOceanToken>()?))
    }
}
