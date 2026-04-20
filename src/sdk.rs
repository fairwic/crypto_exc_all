use crate::account::AccountFacade;
use crate::adapters::ExchangeClient;
use crate::config::SdkConfig;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::market::MarketFacade;
use std::collections::HashMap;

pub struct CryptoSdk {
    clients: HashMap<ExchangeId, ExchangeClient>,
}

impl CryptoSdk {
    pub fn from_env() -> Result<Self> {
        Self::from_config(SdkConfig::from_env())
    }

    pub fn from_config(config: SdkConfig) -> Result<Self> {
        let mut clients = HashMap::new();

        #[cfg(feature = "okx")]
        if let Some(okx_config) = config.okx {
            let client = ExchangeClient::okx(okx_config)?;
            clients.insert(client.exchange_id(), client);
        }

        #[cfg(feature = "binance")]
        if let Some(binance_config) = config.binance {
            let client = ExchangeClient::binance(binance_config)?;
            clients.insert(client.exchange_id(), client);
        }

        #[cfg(feature = "bitget")]
        if let Some(bitget_config) = config.bitget {
            let client = ExchangeClient::bitget(bitget_config)?;
            clients.insert(client.exchange_id(), client);
        }

        Ok(Self { clients })
    }

    pub fn configured_exchanges(&self) -> Vec<ExchangeId> {
        let mut exchanges: Vec<_> = self.clients.keys().copied().collect();
        exchanges.sort_by_key(|exchange| exchange.as_str());
        exchanges
    }

    pub fn market(&self, exchange: ExchangeId) -> Result<MarketFacade<'_>> {
        Ok(MarketFacade::new(self.client(exchange)?))
    }

    pub fn account(&self, exchange: ExchangeId) -> Result<AccountFacade<'_>> {
        Ok(AccountFacade::new(self.client(exchange)?))
    }

    fn client(&self, exchange: ExchangeId) -> Result<&ExchangeClient> {
        self.clients
            .get(&exchange)
            .ok_or(Error::ExchangeNotConfigured(exchange))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BinanceExchangeConfig, BitgetExchangeConfig, OkxExchangeConfig};

    #[test]
    fn builds_sdk_from_explicit_config() {
        let sdk = CryptoSdk::from_config(SdkConfig {
            okx: Some(OkxExchangeConfig {
                api_key: "okx-key".to_string(),
                api_secret: "okx-secret".to_string(),
                passphrase: "okx-pass".to_string(),
                simulated: true,
                api_url: Some("http://127.0.0.1:1".to_string()),
                request_expiration_ms: Some(1_000),
            }),
            binance: Some(BinanceExchangeConfig {
                api_key: "binance-key".to_string(),
                api_secret: "binance-secret".to_string(),
                api_url: Some("http://127.0.0.1:1".to_string()),
                sapi_api_url: None,
                web_api_url: None,
                ws_stream_url: None,
                api_timeout_ms: Some(1_000),
                recv_window_ms: Some(5_000),
                proxy_url: None,
            }),
            bitget: Some(BitgetExchangeConfig {
                api_key: "bitget-key".to_string(),
                api_secret: "bitget-secret".to_string(),
                passphrase: "bitget-pass".to_string(),
                api_url: Some("http://127.0.0.1:1".to_string()),
                api_timeout_ms: Some(1_000),
                proxy_url: None,
                product_type: Some("USDT-FUTURES".to_string()),
            }),
        })
        .unwrap();

        assert_eq!(
            sdk.configured_exchanges(),
            vec![ExchangeId::Binance, ExchangeId::Bitget, ExchangeId::Okx]
        );
        assert!(sdk.market(ExchangeId::Okx).is_ok());
        assert!(sdk.account(ExchangeId::Binance).is_ok());
        assert!(sdk.market(ExchangeId::Bitget).is_ok());
    }
}
