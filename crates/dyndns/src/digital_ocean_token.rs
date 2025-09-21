use crate::types::ValueFromStr;
use color_eyre::eyre::Error;
use secrecy::zeroize::Zeroize;
use secrecy::SecretBox;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct DigitalOceanToken(String);

pub type SecretDigitalOceanToken = SecretBox<DigitalOceanToken>;

impl DigitalOceanToken {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Zeroize for DigitalOceanToken {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl secrecy::CloneableSecret for DigitalOceanToken {}
impl secrecy::SerializableSecret for DigitalOceanToken {}

impl std::str::FromStr for DigitalOceanToken {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(DigitalOceanToken(s.to_owned()))
    }
}

impl ValueFromStr for SecretBox<DigitalOceanToken> {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SecretBox::new(Box::new(s.parse::<DigitalOceanToken>()?)))
    }
}

pub fn parse_secret_token(s: &str) -> Result<SecretBox<DigitalOceanToken>, Error> {
    Ok(SecretBox::new(Box::new(s.parse::<DigitalOceanToken>()?)))
}
