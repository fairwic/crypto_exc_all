use crate::api::{API_SPOT_PUBLIC_PATH, API_SPOT_WALLET_PATH};
use crate::client::BitgetClient;
use crate::error::Error;
use reqwest::Method;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone)]
pub struct BitgetAsset {
    client: BitgetClient,
}

impl BitgetAsset {
    pub fn new(client: BitgetClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self, Error> {
        Ok(Self::new(BitgetClient::from_env()?))
    }

    pub async fn get_coins(&self, coin: Option<&str>) -> Result<Value, Error> {
        let path = format!("{API_SPOT_PUBLIC_PATH}/coins");
        let mut params = Vec::new();
        push_opt(&mut params, "coin", coin);
        self.client
            .send_public_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_deposit_address(
        &self,
        request: DepositAddressRequest,
    ) -> Result<Value, Error> {
        let path = format!("{API_SPOT_WALLET_PATH}/deposit-address");
        self.client
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn get_deposit_records(&self, request: WalletHistoryRequest) -> Result<Value, Error> {
        let path = format!("{API_SPOT_WALLET_PATH}/deposit-records");
        self.client
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn get_withdrawal_records(
        &self,
        request: WalletHistoryRequest,
    ) -> Result<Value, Error> {
        let path = format!("{API_SPOT_WALLET_PATH}/withdrawal-records");
        self.client
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn transfer(&self, request: TransferRequest) -> Result<Value, Error> {
        let path = format!("{API_SPOT_WALLET_PATH}/transfer");
        self.client
            .send_signed_json_request(Method::POST, &path, &[], &request)
            .await
    }

    pub async fn get_transferable_coins(
        &self,
        from_type: &str,
        to_type: &str,
    ) -> Result<Value, Error> {
        let path = format!("{API_SPOT_WALLET_PATH}/transfer-coin-info");
        let params = vec![
            ("fromType", from_type.to_string()),
            ("toType", to_type.to_string()),
        ];
        self.client
            .send_signed_request(Method::GET, &path, &params)
            .await
    }

    pub async fn withdraw<T: Serialize>(&self, request: T) -> Result<Value, Error> {
        let path = format!("{API_SPOT_WALLET_PATH}/withdrawal");
        self.client
            .send_signed_json_request(Method::POST, &path, &[], &request)
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositAddressRequest {
    pub coin: String,
    pub chain: String,
}

impl DepositAddressRequest {
    pub fn new(coin: impl Into<String>, chain: impl Into<String>) -> Self {
        Self {
            coin: coin.into(),
            chain: chain.into(),
        }
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        vec![("chain", self.chain.clone()), ("coin", self.coin.clone())]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletHistoryRequest {
    pub start_time: u64,
    pub end_time: u64,
    pub coin: Option<String>,
    pub client_oid: Option<String>,
    pub id_less_than: Option<String>,
    pub limit: Option<u32>,
}

impl WalletHistoryRequest {
    pub fn new(start_time: u64, end_time: u64) -> Self {
        Self {
            start_time,
            end_time,
            coin: None,
            client_oid: None,
            id_less_than: None,
            limit: None,
        }
    }

    pub fn with_coin(mut self, value: impl Into<String>) -> Self {
        self.coin = Some(value.into());
        self
    }

    pub fn with_client_oid(mut self, value: impl Into<String>) -> Self {
        self.client_oid = Some(value.into());
        self
    }

    pub fn with_id_less_than(mut self, value: impl Into<String>) -> Self {
        self.id_less_than = Some(value.into());
        self
    }

    pub fn with_limit(mut self, value: u32) -> Self {
        self.limit = Some(value);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![
            ("endTime", self.end_time.to_string()),
            ("startTime", self.start_time.to_string()),
        ];
        push_opt_string(&mut params, "coin", self.coin.clone());
        push_opt_string(&mut params, "clientOid", self.client_oid.clone());
        push_opt_string(&mut params, "idLessThan", self.id_less_than.clone());
        push_opt_string(
            &mut params,
            "limit",
            self.limit.map(|value| value.to_string()),
        );
        params
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferRequest {
    pub from_type: String,
    pub to_type: String,
    pub amount: String,
    pub coin: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_oid: Option<String>,
}

impl TransferRequest {
    pub fn new(
        from_type: impl Into<String>,
        to_type: impl Into<String>,
        amount: impl Into<String>,
        coin: impl Into<String>,
    ) -> Self {
        Self {
            from_type: from_type.into(),
            to_type: to_type.into(),
            amount: amount.into(),
            coin: coin.into(),
            symbol: None,
            client_oid: None,
        }
    }

    pub fn with_symbol(mut self, value: impl Into<String>) -> Self {
        self.symbol = Some(value.into());
        self
    }

    pub fn with_client_oid(mut self, value: impl Into<String>) -> Self {
        self.client_oid = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawRequest {
    pub coin: String,
    pub transfer_type: String,
    pub address: String,
    pub size: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_to_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remark: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_oid: Option<String>,
}

impl WithdrawRequest {
    pub fn on_chain(
        coin: impl Into<String>,
        address: impl Into<String>,
        size: impl Into<String>,
    ) -> Self {
        Self::new("on_chain", coin, address, size)
    }

    pub fn internal_transfer(
        coin: impl Into<String>,
        address: impl Into<String>,
        size: impl Into<String>,
    ) -> Self {
        Self::new("internal_transfer", coin, address, size)
    }

    pub fn new(
        transfer_type: impl Into<String>,
        coin: impl Into<String>,
        address: impl Into<String>,
        size: impl Into<String>,
    ) -> Self {
        Self {
            coin: coin.into(),
            transfer_type: transfer_type.into(),
            address: address.into(),
            size: size.into(),
            chain: None,
            inner_to_type: None,
            area_code: None,
            tag: None,
            remark: None,
            client_oid: None,
        }
    }

    pub fn with_chain(mut self, value: impl Into<String>) -> Self {
        self.chain = Some(value.into());
        self
    }

    pub fn with_inner_to_type(mut self, value: impl Into<String>) -> Self {
        self.inner_to_type = Some(value.into());
        self
    }

    pub fn with_area_code(mut self, value: impl Into<String>) -> Self {
        self.area_code = Some(value.into());
        self
    }

    pub fn with_tag(mut self, value: impl Into<String>) -> Self {
        self.tag = Some(value.into());
        self
    }

    pub fn with_remark(mut self, value: impl Into<String>) -> Self {
        self.remark = Some(value.into());
        self
    }

    pub fn with_client_oid(mut self, value: impl Into<String>) -> Self {
        self.client_oid = Some(value.into());
        self
    }
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
