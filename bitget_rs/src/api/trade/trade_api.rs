use crate::api::API_MIX_ORDER_PATH;
use crate::client::BitgetClient;
use crate::error::Error;
use reqwest::Method;
use serde::Serialize;
use serde_json::Value;

#[derive(Clone)]
pub struct BitgetTrade {
    client: BitgetClient,
}

impl BitgetTrade {
    pub fn new(client: BitgetClient) -> Self {
        Self { client }
    }

    pub fn from_env() -> Result<Self, Error> {
        Ok(Self::new(BitgetClient::from_env()?))
    }

    pub async fn place_order(&self, request: NewOrderRequest) -> Result<Value, Error> {
        let path = format!("{API_MIX_ORDER_PATH}/place-order");
        self.client
            .send_signed_json_request(Method::POST, &path, &[], &request)
            .await
    }

    pub async fn place_multiple_orders<T: Serialize>(&self, request: &T) -> Result<Value, Error> {
        let path = format!("{API_MIX_ORDER_PATH}/batch-place-order");
        self.client
            .send_signed_json_request(Method::POST, &path, &[], request)
            .await
    }

    pub async fn cancel_order(&self, request: CancelOrderRequest) -> Result<Value, Error> {
        let path = format!("{API_MIX_ORDER_PATH}/cancel-order");
        self.client
            .send_signed_json_request(Method::POST, &path, &[], &request)
            .await
    }

    pub async fn cancel_multiple_orders<T: Serialize>(&self, request: &T) -> Result<Value, Error> {
        let path = format!("{API_MIX_ORDER_PATH}/cancel-batch-orders");
        self.client
            .send_signed_json_request(Method::POST, &path, &[], request)
            .await
    }

    pub async fn cancel_all_orders(&self, request: CancelAllOrdersRequest) -> Result<Value, Error> {
        let path = format!("{API_MIX_ORDER_PATH}/cancel-all-orders");
        self.client
            .send_signed_json_request(Method::POST, &path, &[], &request)
            .await
    }

    pub async fn modify_order(&self, request: ModifyOrderRequest) -> Result<Value, Error> {
        let path = format!("{API_MIX_ORDER_PATH}/modify-order");
        self.client
            .send_signed_json_request(Method::POST, &path, &[], &request)
            .await
    }

    pub async fn close_positions(&self, request: ClosePositionsRequest) -> Result<Value, Error> {
        let path = format!("{API_MIX_ORDER_PATH}/close-positions");
        self.client
            .send_signed_json_request(Method::POST, &path, &[], &request)
            .await
    }

    pub async fn get_order_detail(
        &self,
        symbol: &str,
        product_type: &str,
        order_id: Option<&str>,
        client_oid: Option<&str>,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_ORDER_PATH}/detail");
        let mut params = vec![
            ("productType", product_type.to_string()),
            ("symbol", symbol.to_string()),
        ];
        push_opt(&mut params, "orderId", order_id);
        push_opt(&mut params, "clientOid", client_oid);
        self.client
            .send_signed_request(Method::GET, &path, &params)
            .await
    }

    pub async fn get_pending_orders(
        &self,
        product_type: &str,
        symbol: Option<&str>,
    ) -> Result<Value, Error> {
        self.get_pending_orders_with(
            OrderQueryRequest::new(product_type).with_optional_symbol(symbol),
        )
        .await
    }

    pub async fn get_pending_orders_with(
        &self,
        request: OrderQueryRequest,
    ) -> Result<Value, Error> {
        let path = format!("{API_MIX_ORDER_PATH}/orders-pending");
        self.client
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn get_order_history(
        &self,
        product_type: &str,
        symbol: Option<&str>,
    ) -> Result<Value, Error> {
        self.get_order_history_with(
            OrderQueryRequest::new(product_type).with_optional_symbol(symbol),
        )
        .await
    }

    pub async fn get_order_history_with(&self, request: OrderQueryRequest) -> Result<Value, Error> {
        let path = format!("{API_MIX_ORDER_PATH}/orders-history");
        self.client
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }

    pub async fn get_fills(
        &self,
        product_type: &str,
        symbol: Option<&str>,
    ) -> Result<Value, Error> {
        self.get_fills_with(OrderQueryRequest::new(product_type).with_optional_symbol(symbol))
            .await
    }

    pub async fn get_fills_with(&self, request: OrderQueryRequest) -> Result<Value, Error> {
        let path = format!("{API_MIX_ORDER_PATH}/fills");
        self.client
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderQueryRequest {
    pub product_type: String,
    pub symbol: Option<String>,
    pub order_id: Option<String>,
    pub client_oid: Option<String>,
    pub status: Option<String>,
    pub id_less_than: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub limit: Option<u32>,
}

impl OrderQueryRequest {
    pub fn new(product_type: impl Into<String>) -> Self {
        Self {
            product_type: product_type.into(),
            symbol: None,
            order_id: None,
            client_oid: None,
            status: None,
            id_less_than: None,
            start_time: None,
            end_time: None,
            limit: None,
        }
    }

    pub fn with_symbol(mut self, value: impl Into<String>) -> Self {
        self.symbol = Some(value.into());
        self
    }

    pub fn with_order_id(mut self, value: impl Into<String>) -> Self {
        self.order_id = Some(value.into());
        self
    }

    pub fn with_client_oid(mut self, value: impl Into<String>) -> Self {
        self.client_oid = Some(value.into());
        self
    }

    pub fn with_status(mut self, value: impl Into<String>) -> Self {
        self.status = Some(value.into());
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

    fn with_optional_symbol(mut self, value: Option<&str>) -> Self {
        self.symbol = value.map(ToOwned::to_owned);
        self
    }

    fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![("productType", self.product_type.clone())];
        push_opt_string(&mut params, "symbol", self.symbol.clone());
        push_opt_string(&mut params, "orderId", self.order_id.clone());
        push_opt_string(&mut params, "clientOid", self.client_oid.clone());
        push_opt_string(&mut params, "status", self.status.clone());
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NewOrderRequest {
    pub symbol: String,
    pub product_type: String,
    pub margin_mode: String,
    pub margin_coin: String,
    pub size: String,
    pub side: String,
    pub order_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_oid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_stop_surplus_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_stop_loss_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stp_mode: Option<String>,
}

impl NewOrderRequest {
    pub fn limit(
        symbol: impl Into<String>,
        product_type: impl Into<String>,
        margin_mode: impl Into<String>,
        margin_coin: impl Into<String>,
        size: impl Into<String>,
        side: impl Into<String>,
        price: impl Into<String>,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            product_type: product_type.into(),
            margin_mode: margin_mode.into(),
            margin_coin: margin_coin.into(),
            size: size.into(),
            side: side.into(),
            order_type: "limit".to_string(),
            price: Some(price.into()),
            trade_side: None,
            force: None,
            client_oid: None,
            reduce_only: None,
            preset_stop_surplus_price: None,
            preset_stop_loss_price: None,
            stp_mode: None,
        }
    }

    pub fn market(
        symbol: impl Into<String>,
        product_type: impl Into<String>,
        margin_mode: impl Into<String>,
        margin_coin: impl Into<String>,
        size: impl Into<String>,
        side: impl Into<String>,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            product_type: product_type.into(),
            margin_mode: margin_mode.into(),
            margin_coin: margin_coin.into(),
            size: size.into(),
            side: side.into(),
            order_type: "market".to_string(),
            price: None,
            trade_side: None,
            force: None,
            client_oid: None,
            reduce_only: None,
            preset_stop_surplus_price: None,
            preset_stop_loss_price: None,
            stp_mode: None,
        }
    }

    pub fn with_trade_side(mut self, value: impl Into<String>) -> Self {
        self.trade_side = Some(value.into());
        self
    }

    pub fn with_force(mut self, value: impl Into<String>) -> Self {
        self.force = Some(value.into());
        self
    }

    pub fn with_client_oid(mut self, value: impl Into<String>) -> Self {
        self.client_oid = Some(value.into());
        self
    }

    pub fn with_reduce_only(mut self, value: impl Into<String>) -> Self {
        self.reduce_only = Some(value.into());
        self
    }

    pub fn with_preset_stop_surplus_price(mut self, value: impl Into<String>) -> Self {
        self.preset_stop_surplus_price = Some(value.into());
        self
    }

    pub fn with_preset_stop_loss_price(mut self, value: impl Into<String>) -> Self {
        self.preset_stop_loss_price = Some(value.into());
        self
    }

    pub fn with_stp_mode(mut self, value: impl Into<String>) -> Self {
        self.stp_mode = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderRequest {
    pub symbol: String,
    pub product_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_coin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_oid: Option<String>,
}

impl CancelOrderRequest {
    pub fn new(symbol: impl Into<String>, product_type: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            product_type: product_type.into(),
            margin_coin: None,
            order_id: None,
            client_oid: None,
        }
    }

    pub fn with_margin_coin(mut self, value: impl Into<String>) -> Self {
        self.margin_coin = Some(value.into());
        self
    }

    pub fn with_order_id(mut self, value: impl Into<String>) -> Self {
        self.order_id = Some(value.into());
        self
    }

    pub fn with_client_oid(mut self, value: impl Into<String>) -> Self {
        self.client_oid = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CancelAllOrdersRequest {
    pub product_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_coin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receive_window: Option<String>,
}

impl CancelAllOrdersRequest {
    pub fn new(product_type: impl Into<String>) -> Self {
        Self {
            product_type: product_type.into(),
            margin_coin: None,
            request_time: None,
            receive_window: None,
        }
    }

    pub fn with_margin_coin(mut self, value: impl Into<String>) -> Self {
        self.margin_coin = Some(value.into());
        self
    }

    pub fn with_request_time(mut self, value: impl Into<String>) -> Self {
        self.request_time = Some(value.into());
        self
    }

    pub fn with_receive_window(mut self, value: impl Into<String>) -> Self {
        self.receive_window = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModifyOrderRequest {
    pub symbol: String,
    pub product_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_coin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_oid: Option<String>,
    pub new_client_oid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_preset_stop_surplus_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_preset_stop_loss_price: Option<String>,
}

impl ModifyOrderRequest {
    pub fn new(
        symbol: impl Into<String>,
        product_type: impl Into<String>,
        new_client_oid: impl Into<String>,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            product_type: product_type.into(),
            margin_coin: None,
            order_id: None,
            client_oid: None,
            new_client_oid: new_client_oid.into(),
            new_size: None,
            new_price: None,
            new_preset_stop_surplus_price: None,
            new_preset_stop_loss_price: None,
        }
    }

    pub fn with_margin_coin(mut self, value: impl Into<String>) -> Self {
        self.margin_coin = Some(value.into());
        self
    }

    pub fn with_order_id(mut self, value: impl Into<String>) -> Self {
        self.order_id = Some(value.into());
        self
    }

    pub fn with_client_oid(mut self, value: impl Into<String>) -> Self {
        self.client_oid = Some(value.into());
        self
    }

    pub fn with_new_size(mut self, value: impl Into<String>) -> Self {
        self.new_size = Some(value.into());
        self
    }

    pub fn with_new_price(mut self, value: impl Into<String>) -> Self {
        self.new_price = Some(value.into());
        self
    }

    pub fn with_new_preset_stop_surplus_price(mut self, value: impl Into<String>) -> Self {
        self.new_preset_stop_surplus_price = Some(value.into());
        self
    }

    pub fn with_new_preset_stop_loss_price(mut self, value: impl Into<String>) -> Self {
        self.new_preset_stop_loss_price = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClosePositionsRequest {
    pub product_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hold_side: Option<String>,
}

impl ClosePositionsRequest {
    pub fn new(product_type: impl Into<String>) -> Self {
        Self {
            product_type: product_type.into(),
            symbol: None,
            hold_side: None,
        }
    }

    pub fn with_symbol(mut self, value: impl Into<String>) -> Self {
        self.symbol = Some(value.into());
        self
    }

    pub fn with_hold_side(mut self, value: impl Into<String>) -> Self {
        self.hold_side = Some(value.into());
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
