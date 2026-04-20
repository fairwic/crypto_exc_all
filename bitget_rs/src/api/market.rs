use crate::api::{API_MIX_MARKET_PATH, API_PUBLIC_PATH};
use crate::client::BitgetClient;
use crate::error::Error;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone)]
pub struct BitgetMarket {
    client: BitgetClient,
}

impl BitgetMarket {
    pub fn new(client: BitgetClient) -> Self {
        Self { client }
    }

    pub fn new_public() -> Result<Self, Error> {
        Ok(Self::new(BitgetClient::new_public()?))
    }

    pub async fn get_ticker(&self, request: TickerRequest) -> Result<Vec<Ticker>, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/ticker");
        self.client
            .send_public_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn get_server_time(&self) -> Result<Value, Error> {
        let path = format!("{API_PUBLIC_PATH}/time");
        self.client
            .send_public_request(Method::GET, &path, &[])
            .await
    }

    pub async fn get_tickers(&self, product_type: &str) -> Result<Vec<Ticker>, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/tickers");
        let params = vec![("productType", product_type.to_string())];
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_contracts(
        &self,
        product_type: &str,
        symbol: Option<&str>,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/contracts");
        let mut params = vec![("productType", product_type.to_string())];
        push_opt(&mut params, "symbol", symbol);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_orderbook(
        &self,
        symbol: &str,
        product_type: &str,
        limit: Option<&str>,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/orderbook");
        let mut params = symbol_product_params(symbol, product_type);
        push_opt(&mut params, "limit", limit);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_merge_depth(
        &self,
        symbol: &str,
        product_type: &str,
        precision: Option<&str>,
        limit: Option<&str>,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/merge-depth");
        let mut params = symbol_product_params(symbol, product_type);
        push_opt(&mut params, "precision", precision);
        push_opt(&mut params, "limit", limit);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_candles(
        &self,
        symbol: &str,
        product_type: &str,
        granularity: &str,
        limit: Option<u32>,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/candles");
        let mut params = symbol_product_params(symbol, product_type);
        params.push(("granularity", granularity.to_string()));
        push_opt_string(&mut params, "limit", limit.map(|value| value.to_string()));
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_history_candles(
        &self,
        symbol: &str,
        product_type: &str,
        granularity: &str,
        limit: Option<u32>,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/history-candles");
        let mut params = symbol_product_params(symbol, product_type);
        params.push(("granularity", granularity.to_string()));
        push_opt_string(&mut params, "limit", limit.map(|value| value.to_string()));
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_symbol_price(&self, symbol: &str, product_type: &str) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/symbol-price");
        self.client
            .send_public_request(
                Method::GET,
                &path,
                &symbol_product_params(symbol, product_type),
            )
            .await
    }

    pub async fn get_current_funding_rate(
        &self,
        product_type: &str,
        symbol: Option<&str>,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/current-fund-rate");
        let mut params = vec![("productType", product_type.to_string())];
        push_opt(&mut params, "symbol", symbol);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_funding_rate_history(
        &self,
        symbol: &str,
        product_type: &str,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/history-fund-rate");
        self.client
            .send_public_request(
                Method::GET,
                &path,
                &symbol_product_params(symbol, product_type),
            )
            .await
    }

    pub async fn get_open_interest(
        &self,
        symbol: &str,
        product_type: &str,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/open-interest");
        self.client
            .send_public_request(
                Method::GET,
                &path,
                &symbol_product_params(symbol, product_type),
            )
            .await
    }

    pub async fn get_open_interest_limit(
        &self,
        product_type: &str,
        symbol: Option<&str>,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/oi-limit");
        let mut params = vec![("productType", product_type.to_string())];
        push_opt(&mut params, "symbol", symbol);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_position_tier(
        &self,
        symbol: &str,
        product_type: &str,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/query-position-lever");
        self.client
            .send_public_request(
                Method::GET,
                &path,
                &symbol_product_params(symbol, product_type),
            )
            .await
    }

    pub async fn get_long_short_ratio(
        &self,
        symbol: &str,
        period: Option<&str>,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/long-short");
        let mut params = vec![("symbol", symbol.to_string())];
        push_opt(&mut params, "period", period);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_account_long_short_ratio(
        &self,
        symbol: &str,
        period: Option<&str>,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/account-long-short");
        let mut params = vec![("symbol", symbol.to_string())];
        push_opt(&mut params, "period", period);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_taker_buy_sell_volume(
        &self,
        symbol: &str,
        period: Option<&str>,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/taker-buy-sell");
        let mut params = vec![("symbol", symbol.to_string())];
        push_opt(&mut params, "period", period);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_exchange_rate(&self) -> Result<Value, Error> {
        let path = format!("{API_MIX_MARKET_PATH}/exchange-rate");
        self.client
            .send_public_request(Method::GET, &path, &[])
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickerRequest {
    pub symbol: String,
    pub product_type: String,
}

impl TickerRequest {
    pub fn new(symbol: impl Into<String>, product_type: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            product_type: product_type.into(),
        }
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("productType", self.product_type.clone()),
            ("symbol", self.symbol.clone()),
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Ticker {
    pub symbol: String,
    #[serde(rename = "lastPr")]
    pub last_price: String,
    #[serde(rename = "askPr")]
    pub ask_price: String,
    #[serde(rename = "bidPr")]
    pub bid_price: String,
    #[serde(default)]
    pub base_volume: String,
    #[serde(default)]
    pub quote_volume: String,
    pub ts: String,
}

fn symbol_product_params(symbol: &str, product_type: &str) -> Vec<(&'static str, String)> {
    vec![
        ("productType", product_type.to_string()),
        ("symbol", symbol.to_string()),
    ]
}

fn push_opt(params: &mut Vec<(&'static str, String)>, key: &'static str, value: Option<&str>) {
    if let Some(value) = value {
        params.push((key, value.to_string()));
    }
}

fn push_opt_string(
    params: &mut Vec<(&'static str, String)>,
    key: &'static str,
    value: Option<String>,
) {
    if let Some(value) = value {
        params.push((key, value));
    }
}
