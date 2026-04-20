use crate::client::BinanceClient;
use crate::error::Error;

pub trait BinanceApiTrait {
    fn new(client: BinanceClient) -> Self
    where
        Self: Sized;

    fn from_env() -> Result<Self, Error>
    where
        Self: Sized,
    {
        Ok(Self::new(BinanceClient::from_env()?))
    }

    fn client(&self) -> &BinanceClient;
}
