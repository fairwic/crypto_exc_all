use super::BinanceTrade;
use crate::api::API_TRADE_V1_PATH;
use crate::api::api_trait::BinanceApiTrait;
use crate::error::Error;
use reqwest::Method;

impl BinanceTrade {
    pub async fn place_algo_order(
        &self,
        request: AlgoOrderRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/algoOrder", API_TRADE_V1_PATH);
        self.client()
            .send_signed_request(Method::POST, &path, &request.to_params())
            .await
    }

    pub async fn cancel_algo_order(
        &self,
        request: AlgoOrderIdRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/algoOrder", API_TRADE_V1_PATH);
        self.client()
            .send_signed_request(Method::DELETE, &path, &request.to_params())
            .await
    }

    pub async fn get_algo_order(
        &self,
        request: AlgoOrderIdRequest,
    ) -> Result<serde_json::Value, Error> {
        let path = format!("{}/algoOrder", API_TRADE_V1_PATH);
        self.client()
            .send_signed_request(Method::GET, &path, &request.to_params())
            .await
    }

    /// Queries all currently open USD-M futures algo orders when `symbol` is `None`.
    pub async fn get_open_algo_orders(
        &self,
        symbol: Option<&str>,
    ) -> Result<serde_json::Value, Error> {
        let params = symbol
            .map(|symbol| vec![("symbol", symbol.to_string())])
            .unwrap_or_default();
        let path = format!("{}/openAlgoOrders", API_TRADE_V1_PATH);
        self.client()
            .send_signed_request(Method::GET, &path, &params)
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlgoOrderRequest {
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub time_in_force: Option<String>,
    pub quantity: Option<String>,
    pub price: Option<String>,
    pub trigger_price: Option<String>,
    pub position_side: Option<String>,
    pub reduce_only: Option<bool>,
    pub close_position: Option<bool>,
    pub working_type: Option<String>,
    pub price_protect: Option<bool>,
    pub client_algo_id: Option<String>,
    pub new_order_resp_type: Option<String>,
}

impl AlgoOrderRequest {
    pub fn conditional(
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
            trigger_price: None,
            position_side: None,
            reduce_only: None,
            close_position: None,
            working_type: None,
            price_protect: None,
            client_algo_id: None,
            new_order_resp_type: None,
        }
    }

    pub fn stop_market(
        symbol: impl Into<String>,
        side: impl Into<String>,
        trigger_price: impl Into<String>,
    ) -> Self {
        Self::conditional(symbol, side, "STOP_MARKET").with_trigger_price(trigger_price)
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

    pub fn with_trigger_price(mut self, value: impl Into<String>) -> Self {
        self.trigger_price = Some(value.into());
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

    pub fn with_close_position(mut self, value: bool) -> Self {
        self.close_position = Some(value);
        self
    }

    pub fn with_working_type(mut self, value: impl Into<String>) -> Self {
        self.working_type = Some(value.into());
        self
    }

    pub fn with_price_protect(mut self, value: bool) -> Self {
        self.price_protect = Some(value);
        self
    }

    pub fn with_client_algo_id(mut self, value: impl Into<String>) -> Self {
        self.client_algo_id = Some(value.into());
        self
    }

    pub fn with_new_order_resp_type(mut self, value: impl Into<String>) -> Self {
        self.new_order_resp_type = Some(value.into());
        self
    }

    pub fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = vec![
            ("algoType", "CONDITIONAL".to_string()),
            ("symbol", self.symbol.clone()),
            ("side", self.side.clone()),
            ("type", self.order_type.clone()),
        ];
        push_optional(&mut params, "timeInForce", self.time_in_force.as_deref());
        push_optional(&mut params, "quantity", self.quantity.as_deref());
        push_optional(&mut params, "price", self.price.as_deref());
        push_optional(&mut params, "triggerPrice", self.trigger_price.as_deref());
        push_optional(&mut params, "positionSide", self.position_side.as_deref());
        push_optional(&mut params, "reduceOnly", self.reduce_only);
        push_optional(&mut params, "closePosition", self.close_position);
        push_optional(&mut params, "workingType", self.working_type.as_deref());
        push_optional(&mut params, "priceProtect", self.price_protect);
        push_optional(&mut params, "clientAlgoId", self.client_algo_id.as_deref());
        push_optional(
            &mut params,
            "newOrderRespType",
            self.new_order_resp_type.as_deref(),
        );
        params
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlgoOrderIdRequest {
    pub algo_id: Option<u64>,
    pub client_algo_id: Option<String>,
}

impl AlgoOrderIdRequest {
    pub fn new() -> Self {
        Self {
            algo_id: None,
            client_algo_id: None,
        }
    }

    pub fn with_algo_id(mut self, value: u64) -> Self {
        self.algo_id = Some(value);
        self
    }

    pub fn with_client_algo_id(mut self, value: impl Into<String>) -> Self {
        self.client_algo_id = Some(value.into());
        self
    }

    pub fn to_params(&self) -> Vec<(&'static str, String)> {
        let mut params = Vec::new();
        push_optional(&mut params, "algoId", self.algo_id);
        push_optional(&mut params, "clientAlgoId", self.client_algo_id.as_deref());
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
