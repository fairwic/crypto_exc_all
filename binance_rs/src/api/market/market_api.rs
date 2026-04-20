use crate::api::api_trait::BinanceApiTrait;
use crate::api::{API_FUTURES_DATA_PATH, API_MARKET_V1_PATH};
use crate::client::BinanceClient;
use crate::dto::market::ServerTime;
use crate::error::Error;
use reqwest::Method;

#[derive(Clone)]
pub struct BinanceMarket {
    client: BinanceClient,
}

impl BinanceApiTrait for BinanceMarket {
    fn new(client: BinanceClient) -> Self {
        Self { client }
    }

    fn client(&self) -> &BinanceClient {
        &self.client
    }
}

impl BinanceMarket {
    pub fn new(client: BinanceClient) -> Self {
        <Self as BinanceApiTrait>::new(client)
    }

    pub fn new_public() -> Result<Self, Error> {
        Ok(Self::new(BinanceClient::new_public()?))
    }

    pub async fn get_server_time(&self) -> Result<ServerTime, Error> {
        let path = format!("{}/time", API_MARKET_V1_PATH);
        self.client
            .send_public_request(Method::GET, &path, &[])
            .await
    }

    pub async fn get_exchange_info(&self) -> Result<serde_json::Value, Error> {
        let path = format!("{}/exchangeInfo", API_MARKET_V1_PATH);
        self.client
            .send_public_request(Method::GET, &path, &[])
            .await
    }

    pub async fn get_depth(
        &self,
        symbol: &str,
        limit: Option<u32>,
    ) -> Result<serde_json::Value, Error> {
        let mut params = vec![("symbol", symbol.to_string())];
        push_optional(&mut params, "limit", limit);

        let path = format!("{}/depth", API_MARKET_V1_PATH);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_klines(&self, request: KlineRequest) -> Result<serde_json::Value, Error> {
        let path = format!("{}/klines", API_MARKET_V1_PATH);
        self.client
            .send_public_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn get_ticker_24hr(&self, symbol: Option<&str>) -> Result<serde_json::Value, Error> {
        let mut params = Vec::new();
        push_optional(&mut params, "symbol", symbol);

        let path = format!("{}/ticker/24hr", API_MARKET_V1_PATH);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_funding_rate_history(
        &self,
        request: FundingRateHistoryRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/fundingRate", API_MARKET_V1_PATH);
        self.client
            .send_public_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn get_mark_price(&self, symbol: Option<&str>) -> Result<serde_json::Value, Error> {
        let mut params = Vec::new();
        push_optional(&mut params, "symbol", symbol);

        let path = format!("{}/premiumIndex", API_MARKET_V1_PATH);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_open_interest(&self, symbol: &str) -> Result<serde_json::Value, Error> {
        let params = vec![("symbol", symbol.to_string())];
        let path = format!("{}/openInterest", API_MARKET_V1_PATH);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_open_interest_statistics(
        &self,
        request: FuturesDataRequest,
    ) -> Result<serde_json::Value, Error> {
        self.get_futures_data("openInterestHist", request).await
    }

    pub async fn get_top_long_short_position_ratio(
        &self,
        request: FuturesDataRequest,
    ) -> Result<serde_json::Value, Error> {
        self.get_futures_data("topLongShortPositionRatio", request)
            .await
    }

    pub async fn get_top_long_short_account_ratio(
        &self,
        request: FuturesDataRequest,
    ) -> Result<serde_json::Value, Error> {
        self.get_futures_data("topLongShortAccountRatio", request)
            .await
    }

    pub async fn get_global_long_short_account_ratio(
        &self,
        request: FuturesDataRequest,
    ) -> Result<serde_json::Value, Error> {
        self.get_futures_data("globalLongShortAccountRatio", request)
            .await
    }

    pub async fn get_taker_buy_sell_volume(
        &self,
        request: FuturesDataRequest,
    ) -> Result<serde_json::Value, Error> {
        self.get_futures_data("takerlongshortRatio", request).await
    }

    async fn get_futures_data(
        &self,
        endpoint: &str,
        request: FuturesDataRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{API_FUTURES_DATA_PATH}/{endpoint}");
        self.client
            .send_public_request(Method::GET, &path, &request.to_params())
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KlineRequest {
    pub symbol: String,
    pub interval: String,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub limit: Option<u32>,
}

impl KlineRequest {
    pub fn new(symbol: impl Into<String>, interval: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            interval: interval.into(),
            start_time: None,
            end_time: None,
            limit: None,
        }
    }

    pub fn with_start_time(mut self, start_time: u64) -> Self {
        self.start_time = Some(start_time);
        self
    }

    pub fn with_end_time(mut self, end_time: u64) -> Self {
        self.end_time = Some(end_time);
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![
            ("symbol", self.symbol.clone()),
            ("interval", self.interval.clone()),
        ];
        push_optional(&mut params, "startTime", self.start_time);
        push_optional(&mut params, "endTime", self.end_time);
        push_optional(&mut params, "limit", self.limit);
        params
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FundingRateHistoryRequest {
    pub symbol: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub limit: Option<u32>,
}

impl FundingRateHistoryRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn with_start_time(mut self, start_time: u64) -> Self {
        self.start_time = Some(start_time);
        self
    }

    pub fn with_end_time(mut self, end_time: u64) -> Self {
        self.end_time = Some(end_time);
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();
        push_optional(&mut params, "symbol", self.symbol.as_deref());
        push_optional(&mut params, "startTime", self.start_time);
        push_optional(&mut params, "endTime", self.end_time);
        push_optional(&mut params, "limit", self.limit);
        params
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuturesDataRequest {
    pub symbol: String,
    pub period: String,
    pub limit: Option<u32>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
}

impl FuturesDataRequest {
    pub fn new(symbol: impl Into<String>, period: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            period: period.into(),
            limit: None,
            start_time: None,
            end_time: None,
        }
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_start_time(mut self, start_time: u64) -> Self {
        self.start_time = Some(start_time);
        self
    }

    pub fn with_end_time(mut self, end_time: u64) -> Self {
        self.end_time = Some(end_time);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![
            ("symbol", self.symbol.clone()),
            ("period", self.period.clone()),
        ];
        push_optional(&mut params, "limit", self.limit);
        push_optional(&mut params, "startTime", self.start_time);
        push_optional(&mut params, "endTime", self.end_time);
        params
    }
}

fn push_optional<T>(params: &mut Vec<(&'static str, String)>, key: &'static str, value: Option<T>)
where
    T: ToString,
{
    if let Some(value) = value {
        params.push((key, value.to_string()));
    }
}
