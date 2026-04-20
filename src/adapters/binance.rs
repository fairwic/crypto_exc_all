use crate::account::Balance;
use crate::config::BinanceExchangeConfig;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use crate::market::Ticker;
use binance_rs::config::{Config as BinanceConfig, Credentials as BinanceCredentials};
use binance_rs::{BinanceAccount, BinanceClient, BinanceMarket};
use serde_json::Value;

pub(crate) struct BinanceAdapter {
    account: BinanceAccount,
    market: BinanceMarket,
}

impl BinanceAdapter {
    pub(crate) fn new(config: BinanceExchangeConfig) -> Result<Self> {
        let mut binance_config = BinanceConfig::from_env();
        if let Some(api_url) = config.api_url {
            binance_config.api_url = api_url;
        }
        if let Some(sapi_api_url) = config.sapi_api_url {
            binance_config.sapi_api_url = sapi_api_url;
        }
        if let Some(web_api_url) = config.web_api_url {
            binance_config.web_api_url = web_api_url;
        }
        if let Some(ws_stream_url) = config.ws_stream_url {
            binance_config.ws_stream_url = ws_stream_url;
        }
        if let Some(api_timeout_ms) = config.api_timeout_ms {
            binance_config.api_timeout_ms = api_timeout_ms;
        }
        if let Some(recv_window_ms) = config.recv_window_ms {
            binance_config.recv_window_ms = recv_window_ms;
        }
        if let Some(proxy_url) = config.proxy_url {
            binance_config.proxy_url = Some(proxy_url);
        }

        let credentials = BinanceCredentials::new(config.api_key, config.api_secret);
        let client = BinanceClient::with_config(Some(credentials), binance_config)
            .map_err(Error::from_binance)?;
        Ok(Self {
            account: BinanceAccount::new(client.clone()),
            market: BinanceMarket::new(client),
        })
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        let exchange = ExchangeId::Binance;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .market
            .get_ticker_24hr(Some(&symbol))
            .await
            .map_err(Error::from_binance)?;
        let object = raw.as_object().ok_or_else(|| Error::Adapter {
            exchange,
            message: "Binance ticker response is not an object".to_string(),
        })?;

        Ok(Ticker {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: symbol,
            last_price: string_field(object, "lastPrice").unwrap_or_default(),
            bid_price: string_field(object, "bidPrice"),
            ask_price: string_field(object, "askPrice"),
            volume_24h: string_field(object, "volume"),
            timestamp: u64_field(object, "closeTime"),
            raw,
        })
    }

    pub(crate) async fn balances(&self) -> Result<Vec<Balance>> {
        let balances = self
            .account
            .get_balance()
            .await
            .map_err(Error::from_binance)?;

        balances
            .into_iter()
            .map(|balance| {
                let raw = serde_json::to_value(&balance)?;
                Ok(Balance {
                    exchange: ExchangeId::Binance,
                    asset: balance.asset,
                    total: balance.balance,
                    available: balance.available_balance,
                    frozen: non_empty(balance.cross_un_pnl),
                    raw,
                })
            })
            .collect()
    }
}

fn string_field(object: &serde_json::Map<String, Value>, field: &str) -> Option<String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn u64_field(object: &serde_json::Map<String, Value>, field: &str) -> Option<u64> {
    object.get(field).and_then(Value::as_u64)
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}
