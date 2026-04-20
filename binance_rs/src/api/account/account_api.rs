use crate::api::api_trait::BinanceApiTrait;
use crate::api::{API_ACCOUNT_V1_PATH, API_ACCOUNT_V2_PATH, API_ACCOUNT_V3_PATH};
use crate::client::BinanceClient;
use crate::dto::account::AccountBalance;
use crate::error::Error;
use reqwest::Method;

#[derive(Clone)]
pub struct BinanceAccount {
    client: BinanceClient,
}

impl BinanceApiTrait for BinanceAccount {
    fn new(client: BinanceClient) -> Self {
        Self { client }
    }

    fn client(&self) -> &BinanceClient {
        &self.client
    }
}

impl BinanceAccount {
    pub fn new(client: BinanceClient) -> Self {
        <Self as BinanceApiTrait>::new(client)
    }

    pub fn from_env() -> Result<Self, Error> {
        <Self as BinanceApiTrait>::from_env()
    }

    pub async fn get_balance(&self) -> Result<Vec<AccountBalance>, Error> {
        let path = format!("{}/balance", API_ACCOUNT_V2_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &[])
            .await
    }

    pub async fn get_account_info(&self) -> Result<serde_json::Value, Error> {
        let path = format!("{}/account", API_ACCOUNT_V3_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &[])
            .await
    }

    pub async fn get_positions(&self, symbol: Option<&str>) -> Result<serde_json::Value, Error> {
        let mut params = Vec::new();
        push_optional(&mut params, "symbol", symbol);

        let path = format!("{}/positionRisk", API_ACCOUNT_V3_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_income_history(
        &self,
        request: IncomeHistoryRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/income", API_ACCOUNT_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn get_commission_rate(&self, symbol: &str) -> Result<serde_json::Value, Error> {
        let params = vec![("symbol", symbol.to_string())];
        let path = format!("{}/commissionRate", API_ACCOUNT_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_account_config(&self) -> Result<serde_json::Value, Error> {
        let path = format!("{}/accountConfig", API_ACCOUNT_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &[])
            .await
    }

    pub async fn get_symbol_config(
        &self,
        symbol: Option<&str>,
    ) -> Result<serde_json::Value, Error> {
        let mut params = Vec::new();
        push_optional(&mut params, "symbol", symbol);

        let path = format!("{}/symbolConfig", API_ACCOUNT_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_order_rate_limit(&self) -> Result<serde_json::Value, Error> {
        let path = format!("{}/rateLimit/order", API_ACCOUNT_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &[])
            .await
    }

    pub async fn get_leverage_brackets(
        &self,
        symbol: Option<&str>,
    ) -> Result<serde_json::Value, Error> {
        let mut params = Vec::new();
        push_optional(&mut params, "symbol", symbol);

        let path = format!("{}/leverageBracket", API_ACCOUNT_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_multi_assets_mode(&self) -> Result<serde_json::Value, Error> {
        let path = format!("{}/multiAssetsMargin", API_ACCOUNT_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &[])
            .await
    }

    pub async fn get_position_mode(&self) -> Result<serde_json::Value, Error> {
        let path = format!("{}/positionSide/dual", API_ACCOUNT_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &[])
            .await
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IncomeHistoryRequest {
    pub symbol: Option<String>,
    pub income_type: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

impl IncomeHistoryRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = Some(symbol.into());
        self
    }

    pub fn with_income_type(mut self, income_type: impl Into<String>) -> Self {
        self.income_type = Some(income_type.into());
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

    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();
        push_optional(&mut params, "symbol", self.symbol.as_deref());
        push_optional(&mut params, "incomeType", self.income_type.as_deref());
        push_optional(&mut params, "startTime", self.start_time);
        push_optional(&mut params, "endTime", self.end_time);
        push_optional(&mut params, "page", self.page);
        push_optional(&mut params, "limit", self.limit);
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
