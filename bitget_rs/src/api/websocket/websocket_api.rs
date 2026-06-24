use crate::config::{Config, Credentials};
use crate::error::Error;
use crate::utils::{current_timestamp_millis, generate_signature};
use serde::de::{DeserializeOwned, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[path = "connection.rs"]
mod connection;
#[path = "reconnect.rs"]
mod reconnect;

pub use connection::BitgetWebsocketSession;
pub use reconnect::{
    BitgetAutoReconnectWebsocketClient, BitgetWebsocketManager, ConnectionState, ReconnectConfig,
    WebsocketMetrics,
};

const WEBSOCKET_CHANNEL_SIZE: usize = 100;
const LOGIN_METHOD: &str = "GET";
const LOGIN_PATH: &str = "/user/verify";

#[derive(Clone)]
pub struct BitgetWebsocket {
    credentials: Option<Credentials>,
    public_url: String,
    private_url: String,
    proxy_url: Option<String>,
}

impl BitgetWebsocket {
    pub fn new(credentials: Credentials, config: Config) -> Result<Self, Error> {
        Ok(Self {
            credentials: Some(credentials),
            public_url: config.ws_public_url,
            private_url: config.ws_private_url,
            proxy_url: config.proxy_url,
        })
    }

    pub fn new_public(config: Config) -> Self {
        Self {
            credentials: None,
            public_url: config.ws_public_url,
            private_url: config.ws_private_url,
            proxy_url: config.proxy_url,
        }
    }

    pub fn from_env() -> Result<Self, Error> {
        Self::new(Credentials::from_env()?, Config::from_env())
    }

    pub fn new_public_with_urls(
        public_url: impl Into<String>,
        private_url: impl Into<String>,
    ) -> Self {
        Self {
            credentials: None,
            public_url: public_url.into(),
            private_url: private_url.into(),
            proxy_url: None,
        }
    }

    pub fn with_proxy_url(mut self, proxy_url: impl Into<String>) -> Self {
        self.proxy_url = Some(proxy_url.into());
        self
    }

    pub fn public_url(&self) -> &str {
        self.public_url.trim_end_matches('/')
    }

    pub fn private_url(&self) -> &str {
        self.private_url.trim_end_matches('/')
    }

    pub async fn connect_public(&self) -> Result<BitgetWebsocketSession, Error> {
        self.connect_url(self.public_url()).await
    }

    pub async fn connect_private(&self) -> Result<BitgetWebsocketSession, Error> {
        self.connect_url(self.private_url()).await
    }

    pub async fn connect_url(&self, url: &str) -> Result<BitgetWebsocketSession, Error> {
        BitgetWebsocketSession::connect_with_proxy(url, self.proxy_url.as_deref()).await
    }

    pub fn login_request(&self) -> Result<Value, Error> {
        self.login_request_at(current_timestamp_millis())
    }

    pub fn login_request_at(&self, timestamp: u64) -> Result<Value, Error> {
        let credentials = self.credentials.as_ref().ok_or(Error::MissingCredentials)?;
        login_request(credentials, timestamp)
    }

    pub fn subscribe_request(channels: &[BitgetWebsocketChannel]) -> String {
        operation_request("subscribe", channels)
    }

    pub fn unsubscribe_request(channels: &[BitgetWebsocketChannel]) -> String {
        operation_request("unsubscribe", channels)
    }

    pub fn place_order_request(
        id: impl Into<String>,
        inst_type: impl Into<String>,
        inst_id: impl Into<String>,
        params: BitgetWebsocketPlaceOrderParams,
    ) -> String {
        trade_request(BitgetWebsocketTradeRequestArg {
            id: id.into(),
            inst_type: inst_type.into(),
            inst_id: inst_id.into(),
            channel: "place-order".to_string(),
            params,
        })
    }

    pub fn cancel_order_request(
        id: impl Into<String>,
        inst_type: impl Into<String>,
        inst_id: impl Into<String>,
        params: BitgetWebsocketCancelOrderParams,
    ) -> String {
        trade_request(BitgetWebsocketTradeRequestArg {
            id: id.into(),
            inst_type: inst_type.into(),
            inst_id: inst_id.into(),
            channel: "cancel-order".to_string(),
            params,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetWebsocketChannel {
    pub inst_type: String,
    pub channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inst_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coin: Option<String>,
}

impl BitgetWebsocketChannel {
    pub fn new(inst_type: impl Into<String>, channel: impl Into<String>) -> Self {
        Self {
            inst_type: inst_type.into(),
            channel: channel.into(),
            inst_id: None,
            coin: None,
        }
    }

    pub fn with_inst_id(mut self, value: impl Into<String>) -> Self {
        self.inst_id = Some(value.into());
        self
    }

    pub fn with_coin(mut self, value: impl Into<String>) -> Self {
        self.coin = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetWebsocketPlaceOrderParams {
    pub order_type: String,
    pub side: String,
    pub size: String,
    pub force: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    pub margin_coin: String,
    pub margin_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_oid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trade_side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_stop_surplus_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_stop_loss_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stp_mode: Option<String>,
}

impl BitgetWebsocketPlaceOrderParams {
    pub fn new(
        order_type: impl Into<String>,
        side: impl Into<String>,
        size: impl Into<String>,
        force: impl Into<String>,
        margin_coin: impl Into<String>,
        margin_mode: impl Into<String>,
    ) -> Self {
        Self {
            order_type: order_type.into(),
            side: side.into(),
            size: size.into(),
            force: force.into(),
            price: None,
            margin_coin: margin_coin.into(),
            margin_mode: margin_mode.into(),
            client_oid: None,
            trade_side: None,
            reduce_only: None,
            preset_stop_surplus_price: None,
            preset_stop_loss_price: None,
            stp_mode: None,
        }
    }

    pub fn limit(
        side: impl Into<String>,
        size: impl Into<String>,
        price: impl Into<String>,
        margin_coin: impl Into<String>,
        margin_mode: impl Into<String>,
        force: impl Into<String>,
    ) -> Self {
        Self::new("limit", side, size, force, margin_coin, margin_mode).with_price(price)
    }

    pub fn market(
        side: impl Into<String>,
        size: impl Into<String>,
        margin_coin: impl Into<String>,
        margin_mode: impl Into<String>,
    ) -> Self {
        Self::new("market", side, size, "", margin_coin, margin_mode)
    }

    pub fn with_price(mut self, value: impl Into<String>) -> Self {
        self.price = Some(value.into());
        self
    }

    pub fn with_client_order_id(mut self, value: impl Into<String>) -> Self {
        self.client_oid = Some(value.into());
        self
    }

    pub fn with_trade_side(mut self, value: impl Into<String>) -> Self {
        self.trade_side = Some(value.into());
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetWebsocketCancelOrderParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_oid: Option<String>,
}

impl BitgetWebsocketCancelOrderParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_order_id(mut self, value: impl Into<String>) -> Self {
        self.order_id = Some(value.into());
        self
    }

    pub fn with_client_order_id(mut self, value: impl Into<String>) -> Self {
        self.client_oid = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct BitgetWebsocketTradeRequestArg<P> {
    id: String,
    inst_type: String,
    channel: String,
    inst_id: String,
    params: P,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetTickerUpdate {
    pub inst_id: Option<String>,
    #[serde(rename = "lastPr")]
    pub last_price: Option<String>,
    #[serde(rename = "bidPr")]
    pub bid_price: Option<String>,
    #[serde(rename = "askPr")]
    pub ask_price: Option<String>,
    #[serde(rename = "bidSz")]
    pub bid_size: Option<String>,
    #[serde(rename = "askSz")]
    pub ask_size: Option<String>,
    pub high24h: Option<String>,
    pub low24h: Option<String>,
    pub change24h: Option<String>,
    pub base_volume: Option<String>,
    pub quote_volume: Option<String>,
    pub mark_price: Option<String>,
    pub index_price: Option<String>,
    pub funding_rate: Option<String>,
    pub next_funding_time: Option<String>,
    pub holding_amount: Option<String>,
    #[serde(rename = "ts")]
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetOrderUpdate {
    pub order_id: Option<String>,
    #[serde(rename = "clientOid")]
    pub client_order_id: Option<String>,
    pub inst_id: Option<String>,
    pub side: Option<String>,
    pub trade_side: Option<String>,
    pub pos_side: Option<String>,
    pub pos_mode: Option<String>,
    pub margin_mode: Option<String>,
    pub margin_coin: Option<String>,
    pub order_type: Option<String>,
    pub force: Option<String>,
    pub price: Option<String>,
    pub size: Option<String>,
    #[serde(rename = "accBaseVolume")]
    pub filled_size: Option<String>,
    #[serde(rename = "priceAvg")]
    pub average_price: Option<String>,
    pub status: Option<String>,
    pub cancel_reason: Option<String>,
    pub fill_price: Option<String>,
    pub trade_id: Option<String>,
    pub fill_time: Option<String>,
    pub fill_fee: Option<String>,
    pub fill_fee_coin: Option<String>,
    pub trade_scope: Option<String>,
    pub total_profits: Option<String>,
    pub leverage: Option<String>,
    pub reduce_only: Option<String>,
    #[serde(rename = "uTime")]
    pub update_time: Option<String>,
    #[serde(rename = "cTime")]
    pub create_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetAccountUpdate {
    pub margin_coin: Option<String>,
    pub frozen: Option<String>,
    pub available: Option<String>,
    pub max_open_pos_available: Option<String>,
    pub max_transfer_out: Option<String>,
    pub equity: Option<String>,
    pub usdt_equity: Option<String>,
    pub crossed_risk_rate: Option<String>,
    #[serde(rename = "unrealizedPL")]
    pub unrealized_pl: Option<String>,
    pub union_total_margin: Option<String>,
    pub union_available: Option<String>,
    pub union_mm: Option<String>,
    pub assets_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetPositionUpdate {
    #[serde(rename = "posId")]
    pub position_id: Option<String>,
    pub inst_id: Option<String>,
    pub margin_coin: Option<String>,
    pub margin_size: Option<String>,
    pub margin_mode: Option<String>,
    pub hold_side: Option<String>,
    pub pos_mode: Option<String>,
    pub total: Option<String>,
    pub available: Option<String>,
    pub frozen: Option<String>,
    pub open_price_avg: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    pub leverage: Option<String>,
    pub achieved_profits: Option<String>,
    #[serde(rename = "unrealizedPL")]
    pub unrealized_pl: Option<String>,
    #[serde(rename = "unrealizedPLR")]
    pub unrealized_pl_ratio: Option<String>,
    pub liquidation_price: Option<String>,
    pub keep_margin_rate: Option<String>,
    pub isolated_margin_rate: Option<String>,
    pub margin_rate: Option<String>,
    pub break_even_price: Option<String>,
    pub total_fee: Option<String>,
    pub deducted_fee: Option<String>,
    pub mark_price: Option<String>,
    pub asset_mode: Option<String>,
    pub auto_margin: Option<String>,
    #[serde(rename = "cTime")]
    pub create_time: Option<String>,
    #[serde(rename = "uTime")]
    pub update_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitgetOrderBookLevel {
    pub price: String,
    pub size: String,
}

impl<'de> Deserialize<'de> for BitgetOrderBookLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = Vec::<String>::deserialize(deserializer)?;
        let price = fields
            .first()
            .cloned()
            .ok_or_else(|| serde::de::Error::custom("missing orderbook level price"))?;
        let size = fields
            .get(1)
            .cloned()
            .ok_or_else(|| serde::de::Error::custom("missing orderbook level size"))?;
        Ok(Self { price, size })
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BitgetOrderBookUpdate {
    pub asks: Vec<BitgetOrderBookLevel>,
    pub bids: Vec<BitgetOrderBookLevel>,
    pub checksum: Option<i64>,
    #[serde(rename = "seq")]
    pub sequence: Option<i64>,
    #[serde(rename = "ts")]
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetTradeUpdate {
    #[serde(rename = "ts")]
    pub timestamp: Option<String>,
    pub price: Option<String>,
    pub size: Option<String>,
    pub side: Option<String>,
    pub trade_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitgetCandleUpdate {
    pub start_time: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub base_volume: String,
    pub quote_volume: String,
    pub usdt_volume: String,
}

impl<'de> Deserialize<'de> for BitgetCandleUpdate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let fields = Vec::<String>::deserialize(deserializer)?;
        let field = |index: usize, name: &str| {
            fields
                .get(index)
                .cloned()
                .ok_or_else(|| serde::de::Error::custom(format!("missing candle {name}")))
        };

        Ok(Self {
            start_time: field(0, "start time")?,
            open: field(1, "open")?,
            high: field(2, "high")?,
            low: field(3, "low")?,
            close: field(4, "close")?,
            base_volume: field(5, "base volume")?,
            quote_volume: field(6, "quote volume")?,
            usdt_volume: field(7, "usdt volume")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetFillFeeDetail {
    pub fee_coin: Option<String>,
    pub deduction: Option<String>,
    pub total_deduction_fee: Option<String>,
    pub total_fee: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetFillUpdate {
    pub order_id: Option<String>,
    #[serde(rename = "clientOid")]
    pub client_order_id: Option<String>,
    pub trade_id: Option<String>,
    pub symbol: Option<String>,
    pub side: Option<String>,
    pub order_type: Option<String>,
    #[serde(rename = "posMode")]
    pub position_mode: Option<String>,
    pub price: Option<String>,
    pub base_volume: Option<String>,
    pub quote_volume: Option<String>,
    pub profit: Option<String>,
    pub trade_side: Option<String>,
    pub trade_scope: Option<String>,
    #[serde(default)]
    pub fee_detail: Vec<BitgetFillFeeDetail>,
    #[serde(rename = "cTime")]
    pub create_time: Option<String>,
    #[serde(rename = "uTime")]
    pub update_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetWebsocketTradeResponseParams {
    pub order_id: Option<String>,
    #[serde(rename = "clientOid")]
    pub client_order_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BitgetWebsocketTradeResponseArg {
    pub id: Option<String>,
    pub inst_type: Option<String>,
    pub channel: String,
    pub inst_id: Option<String>,
    pub params: Option<BitgetWebsocketTradeResponseParams>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BitgetWebsocketEvent {
    Pong,
    Login {
        code: Option<String>,
        msg: Option<String>,
        raw: Value,
    },
    Subscribed {
        arg: BitgetWebsocketChannel,
        raw: Value,
    },
    Unsubscribed {
        arg: BitgetWebsocketChannel,
        raw: Value,
    },
    Error {
        code: Option<String>,
        msg: Option<String>,
        raw: Value,
    },
    Trade {
        code: Option<String>,
        msg: Option<String>,
        args: Vec<BitgetWebsocketTradeResponseArg>,
        raw: Value,
    },
    Ticker {
        action: Option<String>,
        arg: BitgetWebsocketChannel,
        data: Vec<BitgetTickerUpdate>,
        raw: Value,
    },
    Orders {
        action: Option<String>,
        arg: BitgetWebsocketChannel,
        data: Vec<BitgetOrderUpdate>,
        raw: Value,
    },
    Account {
        action: Option<String>,
        arg: BitgetWebsocketChannel,
        data: Vec<BitgetAccountUpdate>,
        raw: Value,
    },
    Positions {
        action: Option<String>,
        arg: BitgetWebsocketChannel,
        data: Vec<BitgetPositionUpdate>,
        raw: Value,
    },
    OrderBook {
        action: Option<String>,
        arg: BitgetWebsocketChannel,
        data: Vec<BitgetOrderBookUpdate>,
        raw: Value,
    },
    Trades {
        action: Option<String>,
        arg: BitgetWebsocketChannel,
        data: Vec<BitgetTradeUpdate>,
        raw: Value,
    },
    Candles {
        action: Option<String>,
        arg: BitgetWebsocketChannel,
        data: Vec<BitgetCandleUpdate>,
        raw: Value,
    },
    Fill {
        action: Option<String>,
        arg: BitgetWebsocketChannel,
        data: Vec<BitgetFillUpdate>,
        raw: Value,
    },
    Data {
        action: Option<String>,
        arg: BitgetWebsocketChannel,
        data: Vec<Value>,
        raw: Value,
    },
    Raw(Value),
}

impl BitgetWebsocketEvent {
    pub fn parse(text: &str) -> Result<Self, Error> {
        if text == "pong" {
            return Ok(Self::Pong);
        }

        let raw: Value = serde_json::from_str(text).map_err(Error::JsonError)?;
        if raw.get("data").is_some() {
            let arg = parse_arg(&raw)?;
            let action = string_field(&raw, "action");
            match arg.channel.as_str() {
                "ticker" => {
                    let data = parse_data_items(&raw)?;
                    return Ok(Self::Ticker {
                        action,
                        arg,
                        data,
                        raw,
                    });
                }
                "orders" => {
                    let data = parse_data_items(&raw)?;
                    return Ok(Self::Orders {
                        action,
                        arg,
                        data,
                        raw,
                    });
                }
                "account" => {
                    let data = parse_data_items(&raw)?;
                    return Ok(Self::Account {
                        action,
                        arg,
                        data,
                        raw,
                    });
                }
                "positions" => {
                    let data = parse_data_items(&raw)?;
                    return Ok(Self::Positions {
                        action,
                        arg,
                        data,
                        raw,
                    });
                }
                channel if channel.starts_with("books") => {
                    let data = parse_data_items(&raw)?;
                    return Ok(Self::OrderBook {
                        action,
                        arg,
                        data,
                        raw,
                    });
                }
                "trade" => {
                    let data = parse_data_items(&raw)?;
                    return Ok(Self::Trades {
                        action,
                        arg,
                        data,
                        raw,
                    });
                }
                channel if channel.starts_with("candle") => {
                    let data = parse_data_items(&raw)?;
                    return Ok(Self::Candles {
                        action,
                        arg,
                        data,
                        raw,
                    });
                }
                "fill" => {
                    let data = parse_data_items(&raw)?;
                    return Ok(Self::Fill {
                        action,
                        arg,
                        data,
                        raw,
                    });
                }
                _ => {}
            }
            let data = match raw.get("data") {
                Some(Value::Array(items)) => items.clone(),
                Some(value) => vec![value.clone()],
                None => Vec::new(),
            };
            return Ok(Self::Data {
                action,
                arg,
                data,
                raw,
            });
        }

        match string_field(&raw, "event").as_deref() {
            Some("trade") => {
                let args = parse_trade_args(&raw)?;
                Ok(Self::Trade {
                    code: string_or_number_field(&raw, "code"),
                    msg: string_or_number_field(&raw, "msg"),
                    args,
                    raw,
                })
            }
            Some("login") => Ok(Self::Login {
                code: string_or_number_field(&raw, "code"),
                msg: string_or_number_field(&raw, "msg"),
                raw,
            }),
            Some("subscribe") => Ok(Self::Subscribed {
                arg: parse_arg(&raw)?,
                raw,
            }),
            Some("unsubscribe") => Ok(Self::Unsubscribed {
                arg: parse_arg(&raw)?,
                raw,
            }),
            Some("error") => Ok(Self::Error {
                code: string_or_number_field(&raw, "code"),
                msg: string_or_number_field(&raw, "msg"),
                raw,
            }),
            _ => Ok(Self::Raw(raw)),
        }
    }
}

fn login_request(credentials: &Credentials, timestamp: u64) -> Result<Value, Error> {
    let timestamp = timestamp.to_string();
    let payload = format!("{timestamp}{LOGIN_METHOD}{LOGIN_PATH}");
    let sign = generate_signature(&credentials.api_secret, &payload)?;
    Ok(json!({
        "op": "login",
        "args": [{
            "apiKey": credentials.api_key,
            "passphrase": credentials.passphrase,
            "timestamp": timestamp,
            "sign": sign,
        }]
    }))
}

fn operation_request(op: &str, channels: &[BitgetWebsocketChannel]) -> String {
    json!({
        "op": op,
        "args": channels,
    })
    .to_string()
}

fn trade_request<P>(arg: BitgetWebsocketTradeRequestArg<P>) -> String
where
    P: Serialize,
{
    json!({
        "op": "trade",
        "args": [arg],
    })
    .to_string()
}

fn parse_arg(raw: &Value) -> Result<BitgetWebsocketChannel, Error> {
    let arg = raw.get("arg").cloned().unwrap_or_else(|| json!({}));
    serde_json::from_value(arg).map_err(Error::JsonError)
}

fn parse_data_items<T>(raw: &Value) -> Result<Vec<T>, Error>
where
    T: DeserializeOwned,
{
    match raw.get("data") {
        Some(Value::Array(items)) => items
            .iter()
            .cloned()
            .map(serde_json::from_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::JsonError),
        Some(value) => serde_json::from_value(value.clone())
            .map(|item| vec![item])
            .map_err(Error::JsonError),
        None => Ok(Vec::new()),
    }
}

fn parse_trade_args(raw: &Value) -> Result<Vec<BitgetWebsocketTradeResponseArg>, Error> {
    match raw.get("arg") {
        Some(Value::Array(items)) => items
            .iter()
            .cloned()
            .map(serde_json::from_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::JsonError),
        Some(value) => serde_json::from_value(value.clone())
            .map(|item| vec![item])
            .map_err(Error::JsonError),
        None => Ok(Vec::new()),
    }
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| match value {
        Value::Null => None,
        Value::String(value) => Some(value),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        other => Some(other.to_string()),
    }))
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn string_or_number_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|value| match value {
        Value::String(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    })
}
