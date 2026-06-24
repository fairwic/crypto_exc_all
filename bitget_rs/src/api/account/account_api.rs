use crate::api::{API_COMMON_PATH, API_MIX_ACCOUNT_PATH, API_MIX_POSITION_PATH};
use crate::client::BitgetClient;
use crate::error::Error;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone)]
pub struct BitgetAccount {
    client: BitgetClient,
}

impl BitgetAccount {
    pub fn new(client: BitgetClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self, Error> {
        Ok(Self::new(BitgetClient::from_env()?))
    }

    pub async fn get_accounts(&self, product_type: &str) -> Result<Vec<Account>, Error> {
        let path = format!("{API_MIX_ACCOUNT_PATH}/accounts");
        let params = vec![("productType", product_type.to_string())];
        self.client
            .send_signed_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_single_account(
        &self,
        symbol: &str,
        product_type: &str,
        margin_coin: &str,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_ACCOUNT_PATH}/account");
        let params = vec![
            ("marginCoin", margin_coin.to_string()),
            ("productType", product_type.to_string()),
            ("symbol", symbol.to_string()),
        ];
        self.client
            .send_signed_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_all_positions(
        &self,
        product_type: &str,
        margin_coin: Option<&str>,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_POSITION_PATH}/all-position");
        let mut params = vec![("productType", product_type.to_string())];
        if let Some(margin_coin) = margin_coin {
            params.push(("marginCoin", margin_coin.to_string()));
        }
        self.client
            .send_signed_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_bills(
        &self,
        product_type: &str,
        coin: Option<&str>,
        business_type: Option<&str>,
    ) -> Result<Value, Error> {
        self.get_account_bills(
            AccountBillRequest::new(product_type)
                .with_optional_coin(coin)
                .with_optional_business_type(business_type),
        )
        .await
    }

    pub async fn get_account_bills(&self, request: AccountBillRequest) -> Result<Value, Error> {
        let path = format!("{API_MIX_ACCOUNT_PATH}/bill");
        self.client
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn set_leverage(
        &self,
        symbol: &str,
        product_type: &str,
        margin_coin: &str,
        leverage: &str,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_ACCOUNT_PATH}/set-leverage");
        let request = SetLeverageRequest {
            symbol,
            product_type,
            margin_coin,
            leverage,
        };
        self.client
            .send_signed_json_request(Method::POST, &path, &[], &request)
            .await
    }

    pub async fn set_margin_mode(
        &self,
        symbol: &str,
        product_type: &str,
        margin_coin: &str,
        margin_mode: &str,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_ACCOUNT_PATH}/set-margin-mode");
        let request = SetMarginModeRequest {
            symbol,
            product_type,
            margin_coin,
            margin_mode,
        };
        self.client
            .send_signed_json_request(Method::POST, &path, &[], &request)
            .await
    }

    pub async fn set_position_mode(
        &self,
        product_type: &str,
        pos_mode: &str,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_ACCOUNT_PATH}/set-position-mode");
        let request = SetPositionModeRequest {
            product_type,
            pos_mode,
        };
        self.client
            .send_signed_json_request(Method::POST, &path, &[], &request)
            .await
    }

    pub async fn set_position_margin(
        &self,
        symbol: &str,
        product_type: &str,
        margin_coin: &str,
        amount: &str,
        hold_side: &str,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_ACCOUNT_PATH}/set-margin");
        let request = SetPositionMarginRequest {
            symbol,
            product_type,
            margin_coin,
            amount,
            hold_side,
        };
        self.client
            .send_signed_json_request(Method::POST, &path, &[], &request)
            .await
    }

    pub async fn set_asset_mode(
        &self,
        product_type: &str,
        asset_mode: &str,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_ACCOUNT_PATH}/set-asset-mode");
        let request = SetAssetModeRequest {
            product_type,
            asset_mode,
        };
        self.client
            .send_signed_json_request(Method::POST, &path, &[], &request)
            .await
    }

    pub async fn get_trade_rate(&self, symbol: &str, business_type: &str) -> Result<Value, Error> {
        let path = format!("{API_COMMON_PATH}/trade-rate");
        let params = vec![
            ("businessType", business_type.to_string()),
            ("symbol", symbol.to_string()),
        ];
        self.client
            .send_signed_request(Method::GET, &path, &params)
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountBillRequest {
    pub product_type: String,
    pub coin: Option<String>,
    pub business_type: Option<String>,
    pub only_funding: Option<String>,
    pub id_less_than: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub limit: Option<u32>,
}

impl AccountBillRequest {
    pub fn new(product_type: impl Into<String>) -> Self {
        Self {
            product_type: product_type.into(),
            coin: None,
            business_type: None,
            only_funding: None,
            id_less_than: None,
            start_time: None,
            end_time: None,
            limit: None,
        }
    }

    pub fn with_coin(mut self, value: impl Into<String>) -> Self {
        self.coin = Some(value.into());
        self
    }

    pub fn with_business_type(mut self, value: impl Into<String>) -> Self {
        self.business_type = Some(value.into());
        self
    }

    pub fn with_only_funding(mut self, value: impl Into<String>) -> Self {
        self.only_funding = Some(value.into());
        self
    }

    pub fn with_id_less_than(mut self, value: impl Into<String>) -> Self {
        self.id_less_than = Some(value.into());
        self
    }

    pub fn with_start_time(mut self, value: u64) -> Self {
        self.start_time = Some(value);
        self
    }

    pub fn with_end_time(mut self, value: u64) -> Self {
        self.end_time = Some(value);
        self
    }

    pub fn with_limit(mut self, value: u32) -> Self {
        self.limit = Some(value);
        self
    }

    fn with_optional_coin(mut self, value: Option<&str>) -> Self {
        self.coin = value.map(ToOwned::to_owned);
        self
    }

    fn with_optional_business_type(mut self, value: Option<&str>) -> Self {
        self.business_type = value.map(ToOwned::to_owned);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![("productType", self.product_type.clone())];
        push_opt_string(&mut params, "coin", self.coin.clone());
        push_opt_string(&mut params, "businessType", self.business_type.clone());
        push_opt_string(&mut params, "onlyFunding", self.only_funding.clone());
        push_opt_string(&mut params, "idLessThan", self.id_less_than.clone());
        push_opt_string(
            &mut params,
            "startTime",
            self.start_time.map(|value| value.to_string()),
        );
        push_opt_string(
            &mut params,
            "endTime",
            self.end_time.map(|value| value.to_string()),
        );
        push_opt_string(
            &mut params,
            "limit",
            self.limit.map(|value| value.to_string()),
        );
        params
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub margin_coin: String,
    #[serde(default)]
    pub locked: String,
    #[serde(default)]
    pub available: String,
    #[serde(default)]
    pub account_equity: String,
    #[serde(default)]
    pub usdt_equity: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetLeverageRequest<'a> {
    symbol: &'a str,
    product_type: &'a str,
    margin_coin: &'a str,
    leverage: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetMarginModeRequest<'a> {
    symbol: &'a str,
    product_type: &'a str,
    margin_coin: &'a str,
    margin_mode: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetPositionModeRequest<'a> {
    product_type: &'a str,
    pos_mode: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetPositionMarginRequest<'a> {
    symbol: &'a str,
    product_type: &'a str,
    margin_coin: &'a str,
    amount: &'a str,
    hold_side: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetAssetModeRequest<'a> {
    product_type: &'a str,
    asset_mode: &'a str,
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
