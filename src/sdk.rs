use crate::account::AccountFacade;
use crate::adapters::ExchangeClient;
use crate::config::SdkConfig;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::fill::FillFacade;
use crate::market::MarketFacade;
use crate::order::OrderFacade;
use crate::platform::PlatformFacade;
use crate::position::PositionFacade;
use crate::private_account_stream::PrivateAccountStreamFacade;
use crate::trade::TradeFacade;
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

        #[cfg(feature = "bybit")]
        if let Some(bybit_config) = config.bybit {
            let client = ExchangeClient::bybit(bybit_config)?;
            clients.insert(client.exchange_id(), client);
        }

        #[cfg(feature = "gate")]
        if let Some(gate_config) = config.gate {
            let client = ExchangeClient::gate(gate_config)?;
            clients.insert(client.exchange_id(), client);
        }

        #[cfg(feature = "hyperliquid")]
        if let Some(hyperliquid_config) = config.hyperliquid {
            let client = ExchangeClient::hyperliquid(hyperliquid_config)?;
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

    pub fn positions(&self, exchange: ExchangeId) -> Result<PositionFacade<'_>> {
        Ok(PositionFacade::new(self.client(exchange)?))
    }

    pub fn trade(&self, exchange: ExchangeId) -> Result<TradeFacade<'_>> {
        Ok(TradeFacade::new(self.client(exchange)?))
    }

    pub fn orders(&self, exchange: ExchangeId) -> Result<OrderFacade<'_>> {
        Ok(OrderFacade::new(self.client(exchange)?))
    }

    pub fn fills(&self, exchange: ExchangeId) -> Result<FillFacade<'_>> {
        Ok(FillFacade::new(self.client(exchange)?))
    }

    pub fn platform(&self, exchange: ExchangeId) -> Result<PlatformFacade<'_>> {
        Ok(PlatformFacade::new(self.client(exchange)?))
    }

    /// 返回统一私有账户流入口；当前仅 OKX 与 Binance 提供实现。
    pub fn private_account_stream(
        &self,
        exchange: ExchangeId,
    ) -> Result<PrivateAccountStreamFacade<'_>> {
        Ok(PrivateAccountStreamFacade::new(self.client(exchange)?))
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
    use crate::config::{
        BinanceExchangeConfig, BitgetExchangeConfig, BybitExchangeConfig, GateExchangeConfig,
        HyperliquidExchangeConfig, OkxExchangeConfig,
    };

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
            bybit: Some(BybitExchangeConfig {
                api_key: "bybit-key".to_string(),
                api_secret: "bybit-secret".to_string(),
                api_url: Some("http://127.0.0.1:1".to_string()),
                api_timeout_ms: Some(1_000),
                recv_window_ms: Some(5_000),
                proxy_url: None,
                category: Some("linear".to_string()),
            }),
            gate: Some(GateExchangeConfig {
                api_key: "gate-key".to_string(),
                api_secret: "gate-secret".to_string(),
                api_url: Some("http://127.0.0.1:1".to_string()),
                api_timeout_ms: Some(1_000),
                proxy_url: None,
                settle: Some("usdt".to_string()),
            }),
            hyperliquid: Some(HyperliquidExchangeConfig {
                api_url: Some("http://127.0.0.1:1".to_string()),
                api_timeout_ms: Some(1_000),
                proxy_url: None,
                user_address: Some("0x0000000000000000000000000000000000000000".to_string()),
            }),
        })
        .unwrap();

        assert_eq!(
            sdk.configured_exchanges(),
            vec![
                ExchangeId::Binance,
                ExchangeId::Bitget,
                ExchangeId::Bybit,
                ExchangeId::Gate,
                ExchangeId::Hyperliquid,
                ExchangeId::Okx
            ]
        );
        assert!(sdk.market(ExchangeId::Okx).is_ok());
        assert!(sdk.account(ExchangeId::Binance).is_ok());
        assert!(sdk.market(ExchangeId::Bitget).is_ok());
        assert!(sdk.market(ExchangeId::Bybit).is_ok());
        assert!(sdk.market(ExchangeId::Gate).is_ok());
        assert!(sdk.market(ExchangeId::Hyperliquid).is_ok());
        assert!(sdk.positions(ExchangeId::Bitget).is_ok());
        assert!(sdk.trade(ExchangeId::Okx).is_ok());
        assert!(sdk.orders(ExchangeId::Bitget).is_ok());
        assert!(sdk.fills(ExchangeId::Binance).is_ok());
        assert!(sdk.private_account_stream(ExchangeId::Okx).is_ok());
        assert!(sdk.private_account_stream(ExchangeId::Binance).is_ok());
    }
}
