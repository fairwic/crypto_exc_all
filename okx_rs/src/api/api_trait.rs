use crate::client::OkxClient;
#[cfg(feature = "full")]
use crate::error::Error;
#[cfg(feature = "full")]
use anyhow::Result;
#[cfg(feature = "full")]
use std::env;

pub trait OkxApiTrait {
    fn new(client: OkxClient) -> Self
    where
        Self: Sized;

    #[cfg(feature = "full")]
    fn from_env() -> Result<Self, Error>
    where
        Self: Sized,
    {
        let is_simulated_trading = env::var("APP_ENV").unwrap_or("local".to_string());
        let client = if is_simulated_trading == "prod" {
            OkxClient::from_env()?
        } else {
            OkxClient::from_env_with_simulated_trading()?
        };
        Ok(Self::new(client))
    }

    // fn from_env() -> Result<Self, Error> where Self: Sized;
    fn client(&self) -> &OkxClient;
}
