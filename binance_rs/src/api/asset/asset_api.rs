use crate::api::api_trait::BinanceApiTrait;
use crate::client::BinanceClient;
use crate::config::{Config, Credentials};
use crate::error::Error;
use reqwest::Method;

#[derive(Clone)]
pub struct BinanceAsset {
    client: BinanceClient,
}

impl BinanceApiTrait for BinanceAsset {
    fn new(client: BinanceClient) -> Self {
        Self { client }
    }

    fn from_env() -> Result<Self, Error> {
        let mut config = Config::from_env();
        config.api_url = config.sapi_api_url.clone();
        let client = BinanceClient::with_config(Some(Credentials::from_env()?), config)?;
        Ok(Self::new(client))
    }

    fn client(&self) -> &BinanceClient {
        &self.client
    }
}

impl BinanceAsset {
    pub fn new(client: BinanceClient) -> Self {
        <Self as BinanceApiTrait>::new(client)
    }

    pub fn from_env() -> Result<Self, Error> {
        <Self as BinanceApiTrait>::from_env()
    }

    pub async fn get_all_coins(&self) -> Result<serde_json::Value, Error> {
        self.client
            .send_signed_request(Method::GET, "/sapi/v1/capital/config/getall", &[])
            .await
    }

    pub async fn get_wallet_balance(
        &self,
        quote_asset: Option<&str>,
    ) -> Result<serde_json::Value, Error> {
        let mut params = Vec::new();
        push_optional(&mut params, "quoteAsset", quote_asset);

        self.client
            .send_signed_request(Method::GET, "/sapi/v1/asset/wallet/balance", &params)
            .await
    }

    pub async fn get_user_assets(
        &self,
        request: UserAssetRequest,
    ) -> Result<serde_json::Value, Error> {
        self.client
            .send_signed_request(
                Method::POST,
                "/sapi/v3/asset/getUserAsset",
                &request.to_params(),
            )
            .await
    }

    pub async fn get_funding_wallet(
        &self,
        request: FundingWalletRequest,
    ) -> Result<serde_json::Value, Error> {
        self.client
            .send_signed_request(
                Method::POST,
                "/sapi/v1/asset/get-funding-asset",
                &request.to_params(),
            )
            .await
    }

    pub async fn transfer(
        &self,
        request: UniversalTransferRequest,
    ) -> Result<serde_json::Value, Error> {
        self.client
            .send_signed_request(
                Method::POST,
                "/sapi/v1/asset/transfer",
                &request.to_params(),
            )
            .await
    }

    pub async fn get_transfer_history(
        &self,
        request: UniversalTransferHistoryRequest,
    ) -> Result<serde_json::Value, Error> {
        self.client
            .send_signed_request(Method::GET, "/sapi/v1/asset/transfer", &request.to_params())
            .await
    }

    pub async fn withdraw(&self, request: WithdrawRequest) -> Result<serde_json::Value, Error> {
        self.client
            .send_signed_request(
                Method::POST,
                "/sapi/v1/capital/withdraw/apply",
                &request.to_params(),
            )
            .await
    }

    pub async fn get_withdraw_history(
        &self,
        request: WithdrawHistoryRequest,
    ) -> Result<serde_json::Value, Error> {
        self.client
            .send_signed_request(
                Method::GET,
                "/sapi/v1/capital/withdraw/history",
                &request.to_params(),
            )
            .await
    }

    pub async fn get_deposit_history(
        &self,
        request: DepositHistoryRequest,
    ) -> Result<serde_json::Value, Error> {
        self.client
            .send_signed_request(
                Method::GET,
                "/sapi/v1/capital/deposit/hisrec",
                &request.to_params(),
            )
            .await
    }

    pub async fn get_deposit_address(
        &self,
        request: DepositAddressRequest,
    ) -> Result<serde_json::Value, Error> {
        self.client
            .send_signed_request(
                Method::GET,
                "/sapi/v1/capital/deposit/address",
                &request.to_params(),
            )
            .await
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserAssetRequest {
    pub asset: Option<String>,
    pub need_btc_valuation: Option<bool>,
}

impl UserAssetRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_asset(mut self, asset: impl Into<String>) -> Self {
        self.asset = Some(asset.into());
        self
    }

    pub fn with_btc_valuation(mut self, need_btc_valuation: bool) -> Self {
        self.need_btc_valuation = Some(need_btc_valuation);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();
        push_optional(&mut params, "asset", self.asset.as_deref());
        push_optional(
            &mut params,
            "needBtcValuation",
            self.need_btc_valuation.map(|value| value.to_string()),
        );
        params
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FundingWalletRequest {
    pub asset: Option<String>,
    pub need_btc_valuation: Option<bool>,
}

impl FundingWalletRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_asset(mut self, asset: impl Into<String>) -> Self {
        self.asset = Some(asset.into());
        self
    }

    pub fn with_btc_valuation(mut self, need_btc_valuation: bool) -> Self {
        self.need_btc_valuation = Some(need_btc_valuation);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();
        push_optional(&mut params, "asset", self.asset.as_deref());
        push_optional(
            &mut params,
            "needBtcValuation",
            self.need_btc_valuation.map(|value| value.to_string()),
        );
        params
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniversalTransferRequest {
    pub transfer_type: String,
    pub asset: String,
    pub amount: String,
    pub from_symbol: Option<String>,
    pub to_symbol: Option<String>,
}

impl UniversalTransferRequest {
    pub fn new(
        transfer_type: impl Into<String>,
        asset: impl Into<String>,
        amount: impl Into<String>,
    ) -> Self {
        Self {
            transfer_type: transfer_type.into(),
            asset: asset.into(),
            amount: amount.into(),
            from_symbol: None,
            to_symbol: None,
        }
    }

    pub fn with_from_symbol(mut self, from_symbol: impl Into<String>) -> Self {
        self.from_symbol = Some(from_symbol.into());
        self
    }

    pub fn with_to_symbol(mut self, to_symbol: impl Into<String>) -> Self {
        self.to_symbol = Some(to_symbol.into());
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![
            ("type", self.transfer_type.clone()),
            ("asset", self.asset.clone()),
            ("amount", self.amount.clone()),
        ];
        push_optional(&mut params, "fromSymbol", self.from_symbol.as_deref());
        push_optional(&mut params, "toSymbol", self.to_symbol.as_deref());
        params
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniversalTransferHistoryRequest {
    pub transfer_type: String,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub current: Option<u32>,
    pub size: Option<u32>,
    pub from_symbol: Option<String>,
    pub to_symbol: Option<String>,
}

impl UniversalTransferHistoryRequest {
    pub fn new(transfer_type: impl Into<String>) -> Self {
        Self {
            transfer_type: transfer_type.into(),
            start_time: None,
            end_time: None,
            current: None,
            size: None,
            from_symbol: None,
            to_symbol: None,
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

    pub fn with_current(mut self, current: u32) -> Self {
        self.current = Some(current);
        self
    }

    pub fn with_size(mut self, size: u32) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_from_symbol(mut self, from_symbol: impl Into<String>) -> Self {
        self.from_symbol = Some(from_symbol.into());
        self
    }

    pub fn with_to_symbol(mut self, to_symbol: impl Into<String>) -> Self {
        self.to_symbol = Some(to_symbol.into());
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![("type", self.transfer_type.clone())];
        push_optional(&mut params, "startTime", self.start_time);
        push_optional(&mut params, "endTime", self.end_time);
        push_optional(&mut params, "current", self.current);
        push_optional(&mut params, "size", self.size);
        push_optional(&mut params, "fromSymbol", self.from_symbol.as_deref());
        push_optional(&mut params, "toSymbol", self.to_symbol.as_deref());
        params
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepositAddressRequest {
    pub coin: String,
    pub network: Option<String>,
    pub amount: Option<String>,
}

impl DepositAddressRequest {
    pub fn new(coin: impl Into<String>) -> Self {
        Self {
            coin: coin.into(),
            network: None,
            amount: None,
        }
    }

    pub fn with_network(mut self, network: impl Into<String>) -> Self {
        self.network = Some(network.into());
        self
    }

    pub fn with_amount(mut self, amount: impl Into<String>) -> Self {
        self.amount = Some(amount.into());
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![("coin", self.coin.clone())];
        push_optional(&mut params, "network", self.network.as_deref());
        push_optional(&mut params, "amount", self.amount.as_deref());
        params
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DepositHistoryRequest {
    pub include_source: Option<bool>,
    pub coin: Option<String>,
    pub status: Option<u32>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
    pub tx_id: Option<String>,
}

impl DepositHistoryRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_include_source(mut self, include_source: bool) -> Self {
        self.include_source = Some(include_source);
        self
    }

    pub fn with_coin(mut self, coin: impl Into<String>) -> Self {
        self.coin = Some(coin.into());
        self
    }

    pub fn with_status(mut self, status: u32) -> Self {
        self.status = Some(status);
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

    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_tx_id(mut self, tx_id: impl Into<String>) -> Self {
        self.tx_id = Some(tx_id.into());
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();
        push_optional(
            &mut params,
            "includeSource",
            self.include_source.map(|value| value.to_string()),
        );
        push_optional(&mut params, "coin", self.coin.as_deref());
        push_optional(&mut params, "status", self.status);
        push_optional(&mut params, "startTime", self.start_time);
        push_optional(&mut params, "endTime", self.end_time);
        push_optional(&mut params, "offset", self.offset);
        push_optional(&mut params, "limit", self.limit);
        push_optional(&mut params, "txId", self.tx_id.as_deref());
        params
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WithdrawHistoryRequest {
    pub coin: Option<String>,
    pub withdraw_order_id: Option<String>,
    pub status: Option<u32>,
    pub offset: Option<u32>,
    pub limit: Option<u32>,
    pub id_list: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
}

impl WithdrawHistoryRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_coin(mut self, coin: impl Into<String>) -> Self {
        self.coin = Some(coin.into());
        self
    }

    pub fn with_withdraw_order_id(mut self, withdraw_order_id: impl Into<String>) -> Self {
        self.withdraw_order_id = Some(withdraw_order_id.into());
        self
    }

    pub fn with_status(mut self, status: u32) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_offset(mut self, offset: u32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_id_list(mut self, id_list: impl Into<String>) -> Self {
        self.id_list = Some(id_list.into());
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
        let mut params = Vec::new();
        push_optional(&mut params, "coin", self.coin.as_deref());
        push_optional(
            &mut params,
            "withdrawOrderId",
            self.withdraw_order_id.as_deref(),
        );
        push_optional(&mut params, "status", self.status);
        push_optional(&mut params, "offset", self.offset);
        push_optional(&mut params, "limit", self.limit);
        push_optional(&mut params, "idList", self.id_list.as_deref());
        push_optional(&mut params, "startTime", self.start_time);
        push_optional(&mut params, "endTime", self.end_time);
        params
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithdrawRequest {
    pub coin: String,
    pub address: String,
    pub amount: String,
    pub withdraw_order_id: Option<String>,
    pub network: Option<String>,
    pub address_tag: Option<String>,
    pub transaction_fee_flag: Option<bool>,
    pub name: Option<String>,
    pub wallet_type: Option<u32>,
}

impl WithdrawRequest {
    pub fn new(
        coin: impl Into<String>,
        address: impl Into<String>,
        amount: impl Into<String>,
    ) -> Self {
        Self {
            coin: coin.into(),
            address: address.into(),
            amount: amount.into(),
            withdraw_order_id: None,
            network: None,
            address_tag: None,
            transaction_fee_flag: None,
            name: None,
            wallet_type: None,
        }
    }

    pub fn with_withdraw_order_id(mut self, withdraw_order_id: impl Into<String>) -> Self {
        self.withdraw_order_id = Some(withdraw_order_id.into());
        self
    }

    pub fn with_network(mut self, network: impl Into<String>) -> Self {
        self.network = Some(network.into());
        self
    }

    pub fn with_address_tag(mut self, address_tag: impl Into<String>) -> Self {
        self.address_tag = Some(address_tag.into());
        self
    }

    pub fn with_transaction_fee_flag(mut self, transaction_fee_flag: bool) -> Self {
        self.transaction_fee_flag = Some(transaction_fee_flag);
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_wallet_type(mut self, wallet_type: u32) -> Self {
        self.wallet_type = Some(wallet_type);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![
            ("coin", self.coin.clone()),
            ("address", self.address.clone()),
            ("amount", self.amount.clone()),
        ];
        push_optional(
            &mut params,
            "withdrawOrderId",
            self.withdraw_order_id.as_deref(),
        );
        push_optional(&mut params, "network", self.network.as_deref());
        push_optional(&mut params, "addressTag", self.address_tag.as_deref());
        push_optional(
            &mut params,
            "transactionFeeFlag",
            self.transaction_fee_flag.map(|value| value.to_string()),
        );
        push_optional(&mut params, "name", self.name.as_deref());
        push_optional(&mut params, "walletType", self.wallet_type);
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
