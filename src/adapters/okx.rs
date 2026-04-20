use crate::account::Balance;
use crate::config::OkxExchangeConfig;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use crate::market::Ticker;
use okx_rs::api::api_trait::OkxApiTrait;
use okx_rs::config::Credentials as OkxCredentials;
use okx_rs::{OkxAccount, OkxClient, OkxMarket};

pub(crate) struct OkxAdapter {
    account: OkxAccount,
    market: OkxMarket,
}

impl OkxAdapter {
    pub(crate) fn new(config: OkxExchangeConfig) -> Result<Self> {
        let credentials = OkxCredentials::new(
            config.api_key,
            config.api_secret,
            config.passphrase,
            if config.simulated { "1" } else { "0" },
        );
        let mut client = OkxClient::new(credentials).map_err(Error::from_okx)?;
        client.set_simulated_trading(if config.simulated { "1" } else { "0" }.to_string());
        if let Some(api_url) = config.api_url {
            client.set_base_url(api_url);
        }
        if let Some(request_expiration_ms) = config.request_expiration_ms {
            client.set_request_expiration(request_expiration_ms);
        }

        Ok(Self {
            account: <OkxAccount as OkxApiTrait>::new(client.clone()),
            market: <OkxMarket as OkxApiTrait>::new(client),
        })
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        let exchange = ExchangeId::Okx;
        let symbol = instrument.symbol_for(exchange);
        let mut tickers = self
            .market
            .get_ticker(&symbol)
            .await
            .map_err(Error::from_okx)?;
        let ticker = tickers.drain(..).next().ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("OKX ticker response is empty for {symbol}"),
        })?;
        let raw = serde_json::to_value(&ticker)?;

        Ok(Ticker {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: symbol,
            last_price: ticker.last,
            bid_price: non_empty(ticker.bid_px),
            ask_price: non_empty(ticker.ask_px),
            volume_24h: non_empty(ticker.vol24h),
            timestamp: ticker.ts.parse::<u64>().ok(),
            raw,
        })
    }

    pub(crate) async fn balances(&self) -> Result<Vec<Balance>> {
        let accounts = self
            .account
            .get_balance(None)
            .await
            .map_err(Error::from_okx)?;
        let mut output = Vec::new();

        for account in accounts {
            for detail in account.details {
                let raw = serde_json::to_value(&detail)?;
                output.push(Balance {
                    exchange: ExchangeId::Okx,
                    asset: detail.ccy,
                    total: detail.eq,
                    available: if detail.avail_bal.is_empty() {
                        detail.avail_eq
                    } else {
                        detail.avail_bal
                    },
                    frozen: non_empty(detail.frozen_bal),
                    raw,
                });
            }
        }

        Ok(output)
    }
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}
