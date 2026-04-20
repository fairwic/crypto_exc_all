use crate::account::Balance;
use crate::config::BitgetExchangeConfig;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use crate::market::Ticker;
use bitget_rs::api::market::TickerRequest;
use bitget_rs::config::{Config as BitgetConfig, Credentials as BitgetCredentials};
use bitget_rs::{BitgetAccount, BitgetClient, BitgetMarket};

const DEFAULT_PRODUCT_TYPE: &str = "USDT-FUTURES";

pub(crate) struct BitgetAdapter {
    account: BitgetAccount,
    market: BitgetMarket,
    product_type: String,
}

impl BitgetAdapter {
    pub(crate) fn new(config: BitgetExchangeConfig) -> Result<Self> {
        let mut bitget_config = BitgetConfig::from_env();
        if let Some(api_url) = config.api_url {
            bitget_config.api_url = api_url;
        }
        if let Some(api_timeout_ms) = config.api_timeout_ms {
            bitget_config.api_timeout_ms = api_timeout_ms;
        }
        if let Some(proxy_url) = config.proxy_url {
            bitget_config.proxy_url = Some(proxy_url);
        }

        let product_type = config
            .product_type
            .unwrap_or_else(|| DEFAULT_PRODUCT_TYPE.to_string());
        let credentials =
            BitgetCredentials::new(config.api_key, config.api_secret, config.passphrase);
        let client = BitgetClient::with_config(Some(credentials), bitget_config)
            .map_err(Error::from_bitget)?;

        Ok(Self {
            account: BitgetAccount::new(client.clone()),
            market: BitgetMarket::new(client),
            product_type,
        })
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        let exchange = ExchangeId::Bitget;
        let symbol = instrument.symbol_for(exchange);
        let mut tickers = self
            .market
            .get_ticker(TickerRequest::new(&symbol, &self.product_type))
            .await
            .map_err(Error::from_bitget)?;
        let ticker = tickers.drain(..).next().ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("Bitget ticker response is empty for {symbol}"),
        })?;
        let raw = serde_json::to_value(&ticker)?;

        Ok(Ticker {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: symbol,
            last_price: ticker.last_price,
            bid_price: non_empty(ticker.bid_price),
            ask_price: non_empty(ticker.ask_price),
            volume_24h: non_empty(ticker.quote_volume).or_else(|| non_empty(ticker.base_volume)),
            timestamp: ticker.ts.parse::<u64>().ok(),
            raw,
        })
    }

    pub(crate) async fn balances(&self) -> Result<Vec<Balance>> {
        let accounts = self
            .account
            .get_accounts(&self.product_type)
            .await
            .map_err(Error::from_bitget)?;

        accounts
            .into_iter()
            .map(|account| {
                let raw = serde_json::to_value(&account)?;
                Ok(Balance {
                    exchange: ExchangeId::Bitget,
                    asset: account.margin_coin,
                    total: first_non_empty(&account.account_equity, &account.usdt_equity)
                        .unwrap_or_default(),
                    available: account.available,
                    frozen: non_empty(account.locked),
                    raw,
                })
            })
            .collect()
    }
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

fn first_non_empty(primary: &str, fallback: &str) -> Option<String> {
    if !primary.is_empty() {
        Some(primary.to_string())
    } else if !fallback.is_empty() {
        Some(fallback.to_string())
    } else {
        None
    }
}
