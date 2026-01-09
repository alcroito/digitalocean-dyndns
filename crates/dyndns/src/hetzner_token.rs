use crate::types::ValueFromStr;
use color_eyre::eyre::Error;
use secrecy::zeroize::Zeroize;
use secrecy::SecretBox;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct HetznerToken(String);

pub type SecretHetznerToken = SecretBox<HetznerToken>;

impl HetznerToken {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Zeroize for HetznerToken {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl secrecy::CloneableSecret for HetznerToken {}
impl secrecy::SerializableSecret for HetznerToken {}

impl std::str::FromStr for HetznerToken {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(HetznerToken(s.to_owned()))
    }
}

impl ValueFromStr for SecretBox<HetznerToken> {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SecretBox::new(Box::new(s.parse::<HetznerToken>()?)))
    }
}

pub fn parse_secret_token(s: &str) -> Result<SecretBox<HetznerToken>, Error> {
    Ok(SecretBox::new(Box::new(s.parse::<HetznerToken>()?)))
}
