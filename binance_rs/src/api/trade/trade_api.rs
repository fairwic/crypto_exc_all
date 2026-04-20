use crate::api::API_TRADE_V1_PATH;
use crate::api::api_trait::BinanceApiTrait;
use crate::client::BinanceClient;
use crate::error::Error;
use reqwest::Method;

#[derive(Clone)]
pub struct BinanceTrade {
    client: BinanceClient,
}

impl BinanceApiTrait for BinanceTrade {
    fn new(client: BinanceClient) -> Self {
        Self { client }
    }

    fn client(&self) -> &BinanceClient {
        &self.client
    }
}

impl BinanceTrade {
    pub fn new(client: BinanceClient) -> Self {
        <Self as BinanceApiTrait>::new(client)
    }

    pub fn from_env() -> Result<Self, Error> {
        <Self as BinanceApiTrait>::from_env()
    }

    pub async fn place_order(&self, request: NewOrderRequest) -> Result<serde_json::Value, Error> {
        let path = format!("{}/order", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::POST, &path, &request.to_params())
            .await
    }

    pub async fn test_order(&self, request: NewOrderRequest) -> Result<serde_json::Value, Error> {
        let path = format!("{}/order/test", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::POST, &path, &request.to_params())
            .await
    }

    pub async fn cancel_order(&self, request: OrderIdRequest) -> Result<serde_json::Value, Error> {
        let path = format!("{}/order", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::DELETE, &path, &request.to_params())
            .await
    }

    pub async fn get_order(&self, request: OrderIdRequest) -> Result<serde_json::Value, Error> {
        let path = format!("{}/order", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn get_open_orders(&self, symbol: Option<&str>) -> Result<serde_json::Value, Error> {
        let mut params = Vec::new();
        push_optional(&mut params, "symbol", symbol);

        let path = format!("{}/openOrders", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_all_orders(
        &self,
        request: OrderListRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/allOrders", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn get_user_trades(
        &self,
        request: OrderListRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/userTrades", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn change_leverage(
        &self,
        request: ChangeLeverageRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/leverage", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::POST, &path, &request.to_params())
            .await
    }

    pub async fn place_multiple_orders(
        &self,
        request: BatchOrdersRequest<NewOrderRequest>,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/batchOrders", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::POST, &path, &request.to_params())
            .await
    }

    pub async fn modify_order(
        &self,
        request: ModifyOrderRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/order", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::PUT, &path, &request.to_params())
            .await
    }

    pub async fn modify_multiple_orders(
        &self,
        request: BatchOrdersRequest<ModifyOrderRequest>,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/batchOrders", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::PUT, &path, &request.to_params())
            .await
    }

    pub async fn cancel_multiple_orders(
        &self,
        request: CancelMultipleOrdersRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/batchOrders", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::DELETE, &path, &request.to_params())
            .await
    }

    pub async fn cancel_all_open_orders(&self, symbol: &str) -> Result<serde_json::Value, Error> {
        let params = vec![("symbol", symbol.to_string())];
        let path = format!("{}/allOpenOrders", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::DELETE, &path, &params)
            .await
    }

    pub async fn get_open_order(
        &self,
        request: OrderIdRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/openOrder", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn change_margin_type(
        &self,
        request: ChangeMarginTypeRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/marginType", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::POST, &path, &request.to_params())
            .await
    }

    pub async fn change_position_mode(
        &self,
        request: ChangePositionModeRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/positionSide/dual", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::POST, &path, &request.to_params())
            .await
    }

    pub async fn change_multi_assets_mode(
        &self,
        request: ChangeMultiAssetsModeRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/multiAssetsMargin", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::POST, &path, &request.to_params())
            .await
    }

    pub async fn modify_position_margin(
        &self,
        request: ModifyPositionMarginRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/positionMargin", API_TRADE_V1_PATH);
        self.client
            .send_signed_request(Method::POST, &path, &request.to_params())
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewOrderRequest {
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub time_in_force: Option<String>,
    pub quantity: Option<String>,
    pub price: Option<String>,
    pub position_side: Option<String>,
    pub reduce_only: Option<bool>,
    pub new_client_order_id: Option<String>,
    pub new_order_resp_type: Option<String>,
}

impl NewOrderRequest {
    pub fn new(
        symbol: impl Into<String>,
        side: impl Into<String>,
        order_type: impl Into<String>,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            side: side.into(),
            order_type: order_type.into(),
            time_in_force: None,
            quantity: None,
            price: None,
            position_side: None,
            reduce_only: None,
            new_client_order_id: None,
            new_order_resp_type: None,
        }
    }

    pub fn limit(
        symbol: impl Into<String>,
        side: impl Into<String>,
        quantity: impl Into<String>,
        price: impl Into<String>,
        time_in_force: impl Into<String>,
    ) -> Self {
        Self::new(symbol, side, "LIMIT")
            .with_time_in_force(time_in_force)
            .with_quantity(quantity)
            .with_price(price)
    }

    pub fn market(
        symbol: impl Into<String>,
        side: impl Into<String>,
        quantity: impl Into<String>,
    ) -> Self {
        Self::new(symbol, side, "MARKET").with_quantity(quantity)
    }

    pub fn with_time_in_force(mut self, value: impl Into<String>) -> Self {
        self.time_in_force = Some(value.into());
        self
    }

    pub fn with_quantity(mut self, value: impl Into<String>) -> Self {
        self.quantity = Some(value.into());
        self
    }

    pub fn with_price(mut self, value: impl Into<String>) -> Self {
        self.price = Some(value.into());
        self
    }

    pub fn with_position_side(mut self, value: impl Into<String>) -> Self {
        self.position_side = Some(value.into());
        self
    }

    pub fn with_reduce_only(mut self, value: bool) -> Self {
        self.reduce_only = Some(value);
        self
    }

    pub fn with_new_client_order_id(mut self, value: impl Into<String>) -> Self {
        self.new_client_order_id = Some(value.into());
        self
    }

    pub fn with_new_order_resp_type(mut self, value: impl Into<String>) -> Self {
        self.new_order_resp_type = Some(value.into());
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![
            ("symbol", self.symbol.clone()),
            ("side", self.side.clone()),
            ("type", self.order_type.clone()),
        ];
        push_optional(&mut params, "timeInForce", self.time_in_force.as_deref());
        push_optional(&mut params, "quantity", self.quantity.as_deref());
        push_optional(&mut params, "price", self.price.as_deref());
        push_optional(&mut params, "positionSide", self.position_side.as_deref());
        push_optional(
            &mut params,
            "reduceOnly",
            self.reduce_only.map(|v| v.to_string()),
        );
        push_optional(
            &mut params,
            "newClientOrderId",
            self.new_client_order_id.as_deref(),
        );
        push_optional(
            &mut params,
            "newOrderRespType",
            self.new_order_resp_type.as_deref(),
        );
        params
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderIdRequest {
    pub symbol: String,
    pub order_id: Option<u64>,
    pub orig_client_order_id: Option<String>,
}

impl OrderIdRequest {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            order_id: None,
            orig_client_order_id: None,
        }
    }

    pub fn with_order_id(mut self, order_id: u64) -> Self {
        self.order_id = Some(order_id);
        self
    }

    pub fn with_orig_client_order_id(mut self, orig_client_order_id: impl Into<String>) -> Self {
        self.orig_client_order_id = Some(orig_client_order_id.into());
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![("symbol", self.symbol.clone())];
        push_optional(&mut params, "orderId", self.order_id);
        push_optional(
            &mut params,
            "origClientOrderId",
            self.orig_client_order_id.as_deref(),
        );
        params
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderListRequest {
    pub symbol: String,
    pub order_id: Option<u64>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub from_id: Option<u64>,
    pub limit: Option<u32>,
}

impl OrderListRequest {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            order_id: None,
            start_time: None,
            end_time: None,
            from_id: None,
            limit: None,
        }
    }

    pub fn with_order_id(mut self, order_id: u64) -> Self {
        self.order_id = Some(order_id);
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

    pub fn with_from_id(mut self, from_id: u64) -> Self {
        self.from_id = Some(from_id);
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![("symbol", self.symbol.clone())];
        push_optional(&mut params, "orderId", self.order_id);
        push_optional(&mut params, "startTime", self.start_time);
        push_optional(&mut params, "endTime", self.end_time);
        push_optional(&mut params, "fromId", self.from_id);
        push_optional(&mut params, "limit", self.limit);
        params
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeLeverageRequest {
    pub symbol: String,
    pub leverage: u32,
}

impl ChangeLeverageRequest {
    pub fn new(symbol: impl Into<String>, leverage: u32) -> Self {
        Self {
            symbol: symbol.into(),
            leverage,
        }
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("symbol", self.symbol.clone()),
            ("leverage", self.leverage.to_string()),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifyOrderRequest {
    pub symbol: String,
    pub side: String,
    pub quantity: String,
    pub price: String,
    pub order_id: Option<u64>,
    pub orig_client_order_id: Option<String>,
    pub price_match: Option<String>,
}

impl ModifyOrderRequest {
    pub fn new(
        symbol: impl Into<String>,
        side: impl Into<String>,
        quantity: impl Into<String>,
        price: impl Into<String>,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            side: side.into(),
            quantity: quantity.into(),
            price: price.into(),
            order_id: None,
            orig_client_order_id: None,
            price_match: None,
        }
    }

    pub fn with_order_id(mut self, order_id: u64) -> Self {
        self.order_id = Some(order_id);
        self
    }

    pub fn with_orig_client_order_id(mut self, orig_client_order_id: impl Into<String>) -> Self {
        self.orig_client_order_id = Some(orig_client_order_id.into());
        self
    }

    pub fn with_price_match(mut self, price_match: impl Into<String>) -> Self {
        self.price_match = Some(price_match.into());
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![
            ("symbol", self.symbol.clone()),
            ("side", self.side.clone()),
            ("quantity", self.quantity.clone()),
            ("price", self.price.clone()),
        ];
        push_optional(&mut params, "orderId", self.order_id);
        push_optional(
            &mut params,
            "origClientOrderId",
            self.orig_client_order_id.as_deref(),
        );
        push_optional(&mut params, "priceMatch", self.price_match.as_deref());
        params
    }
}

pub trait BatchOrderParams {
    fn batch_order_params(&self) -> Vec<(&'static str, String)>;
}

impl BatchOrderParams for NewOrderRequest {
    fn batch_order_params(&self) -> Vec<(&'static str, String)> {
        self.to_params()
    }
}

impl BatchOrderParams for ModifyOrderRequest {
    fn batch_order_params(&self) -> Vec<(&'static str, String)> {
        self.to_params()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchOrdersRequest<T> {
    pub orders: Vec<T>,
}

impl<T> BatchOrdersRequest<T> {
    pub fn new(orders: Vec<T>) -> Self {
        Self { orders }
    }
}

impl<T> BatchOrdersRequest<T>
where
    T: BatchOrderParams,
{
    fn to_params(&self) -> Vec<(&'static str, String)> {
        let orders = self
            .orders
            .iter()
            .map(batch_params_to_json)
            .collect::<Vec<_>>();
        vec![(
            "batchOrders",
            serde_json::to_string(&orders).unwrap_or_else(|_| "[]".to_string()),
        )]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelMultipleOrdersRequest {
    pub symbol: String,
    pub order_id_list: Option<Vec<u64>>,
    pub orig_client_order_id_list: Option<Vec<String>>,
}

impl CancelMultipleOrdersRequest {
    pub fn new(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            order_id_list: None,
            orig_client_order_id_list: None,
        }
    }

    pub fn with_order_ids(mut self, order_ids: Vec<u64>) -> Self {
        self.order_id_list = Some(order_ids);
        self
    }

    pub fn with_orig_client_order_ids(
        mut self,
        orig_client_order_ids: Vec<impl Into<String>>,
    ) -> Self {
        self.orig_client_order_id_list =
            Some(orig_client_order_ids.into_iter().map(Into::into).collect());
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![("symbol", self.symbol.clone())];
        if let Some(order_ids) = &self.order_id_list {
            params.push((
                "orderIdList",
                serde_json::to_string(order_ids).unwrap_or_else(|_| "[]".to_string()),
            ));
        }
        if let Some(client_ids) = &self.orig_client_order_id_list {
            params.push((
                "origClientOrderIdList",
                serde_json::to_string(client_ids).unwrap_or_else(|_| "[]".to_string()),
            ));
        }
        params
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeMarginTypeRequest {
    pub symbol: String,
    pub margin_type: String,
}

impl ChangeMarginTypeRequest {
    pub fn new(symbol: impl Into<String>, margin_type: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            margin_type: margin_type.into(),
        }
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        vec![
            ("symbol", self.symbol.clone()),
            ("marginType", self.margin_type.clone()),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePositionModeRequest {
    pub dual_side_position: bool,
}

impl ChangePositionModeRequest {
    pub fn new(dual_side_position: bool) -> Self {
        Self { dual_side_position }
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        vec![("dualSidePosition", self.dual_side_position.to_string())]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeMultiAssetsModeRequest {
    pub multi_assets_margin: bool,
}

impl ChangeMultiAssetsModeRequest {
    pub fn new(multi_assets_margin: bool) -> Self {
        Self {
            multi_assets_margin,
        }
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        vec![("multiAssetsMargin", self.multi_assets_margin.to_string())]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifyPositionMarginRequest {
    pub symbol: String,
    pub amount: String,
    pub margin_action_type: u32,
    pub position_side: Option<String>,
}

impl ModifyPositionMarginRequest {
    pub fn new(
        symbol: impl Into<String>,
        amount: impl Into<String>,
        margin_action_type: u32,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            amount: amount.into(),
            margin_action_type,
            position_side: None,
        }
    }

    pub fn with_position_side(mut self, position_side: impl Into<String>) -> Self {
        self.position_side = Some(position_side.into());
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![
            ("symbol", self.symbol.clone()),
            ("amount", self.amount.clone()),
            ("type", self.margin_action_type.to_string()),
        ];
        push_optional(&mut params, "positionSide", self.position_side.as_deref());
        params
    }
}

fn batch_params_to_json<T>(request: &T) -> serde_json::Value
where
    T: BatchOrderParams,
{
    request
        .batch_order_params()
        .into_iter()
        .map(|(key, value)| (key.to_string(), serde_json::Value::String(value)))
        .collect()
}

fn push_optional<T>(params: &mut Vec<(&'static str, String)>, key: &'static str, value: Option<T>)
where
    T: ToString,
{
    if let Some(value) = value {
        params.push((key, value.to_string()));
    }
}
