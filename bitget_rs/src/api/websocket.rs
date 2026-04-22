use crate::config::{Config, Credentials};
use crate::error::Error;
use crate::utils::{current_timestamp_millis, generate_signature};
use futures_util::{SinkExt, StreamExt};
use serde::de::{DeserializeOwned, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::env;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::{interval, sleep, timeout};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::client::Response;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, client_async_tls_with_config, connect_async,
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

pub struct BitgetWebsocketSession {
    tx: mpsc::Sender<Message>,
    rx: mpsc::Receiver<BitgetWebsocketEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReconnectConfig {
    pub reconnect_interval: Duration,
    pub max_reconnect_attempts: u32,
    pub ping_interval: Duration,
    pub message_timeout: Duration,
    pub backoff_factor: f64,
    pub max_backoff: Duration,
}

impl ReconnectConfig {
    pub fn new(reconnect_interval: Duration, max_reconnect_attempts: u32) -> Self {
        Self {
            reconnect_interval,
            max_reconnect_attempts,
            ping_interval: Duration::from_secs(30),
            message_timeout: Duration::from_secs(90),
            backoff_factor: 1.5,
            max_backoff: Duration::from_secs(60),
        }
    }

    pub fn with_ping_interval(mut self, value: Duration) -> Self {
        self.ping_interval = value;
        self
    }

    pub fn with_message_timeout(mut self, value: Duration) -> Self {
        self.message_timeout = value;
        self
    }

    pub fn with_backoff_factor(mut self, value: f64) -> Self {
        self.backoff_factor = value;
        self
    }

    pub fn with_max_backoff(mut self, value: Duration) -> Self {
        self.max_backoff = value;
        self
    }
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self::new(Duration::from_secs(5), 10)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Stopped,
}

#[derive(Debug, Clone, Default)]
pub struct WebsocketMetrics {
    pub connected_at: Option<Instant>,
    pub last_message_at: Option<Instant>,
    pub messages_received: u64,
    pub reconnects: u64,
    pub connection_attempts: u64,
    pub last_error: Option<String>,
}

pub struct BitgetWebsocketManager {
    urls: Vec<String>,
    config: ReconnectConfig,
    subscriptions: Vec<BitgetWebsocketChannel>,
    login_credentials: Option<Credentials>,
    proxy_url: Option<String>,
    command_tx: Option<mpsc::Sender<BitgetWebsocketCommand>>,
    state_tx: watch::Sender<ConnectionState>,
    metrics_tx: watch::Sender<WebsocketMetrics>,
    stop_tx: Option<watch::Sender<bool>>,
    task: Option<JoinHandle<()>>,
}

enum BitgetWebsocketCommand {
    Subscribe(BitgetWebsocketChannel),
    Unsubscribe(BitgetWebsocketChannel),
}

impl BitgetWebsocketManager {
    pub fn new(url: impl Into<String>, config: ReconnectConfig) -> Self {
        let (state_tx, _) = watch::channel(ConnectionState::Disconnected);
        let (metrics_tx, _) = watch::channel(WebsocketMetrics::default());
        Self {
            urls: build_websocket_url_pool(url.into()),
            config,
            subscriptions: Vec::new(),
            login_credentials: None,
            proxy_url: None,
            command_tx: None,
            state_tx,
            metrics_tx,
            stop_tx: None,
            task: None,
        }
    }

    pub fn with_proxy_url(mut self, proxy_url: impl Into<String>) -> Self {
        self.proxy_url = Some(proxy_url.into());
        self
    }

    pub fn with_fallback_urls<I, S>(mut self, urls: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for url in urls {
            let url = url.into();
            push_websocket_url_candidate(&mut self.urls, &url);
        }
        self
    }

    pub fn urls(&self) -> &[String] {
        &self.urls
    }

    pub fn with_login_credentials(mut self, credentials: Credentials) -> Self {
        self.login_credentials = Some(credentials);
        self
    }

    pub fn add_subscription(&mut self, channel: BitgetWebsocketChannel) {
        if !self.subscriptions.contains(&channel) {
            self.subscriptions.push(channel);
        }
    }

    pub async fn subscribe(&mut self, channel: BitgetWebsocketChannel) -> Result<(), Error> {
        let inserted = if self.subscriptions.contains(&channel) {
            false
        } else {
            self.subscriptions.push(channel.clone());
            true
        };

        if inserted {
            self.send_command(BitgetWebsocketCommand::Subscribe(channel))
                .await?;
        }

        Ok(())
    }

    pub async fn unsubscribe(&mut self, channel: BitgetWebsocketChannel) -> Result<(), Error> {
        let previous_len = self.subscriptions.len();
        self.subscriptions.retain(|item| item != &channel);

        if self.subscriptions.len() != previous_len {
            self.send_command(BitgetWebsocketCommand::Unsubscribe(channel))
                .await?;
        }

        Ok(())
    }

    pub fn subscriptions(&self) -> &[BitgetWebsocketChannel] {
        &self.subscriptions
    }

    pub fn connection_state(&self) -> ConnectionState {
        self.state_tx.borrow().clone()
    }

    pub fn metrics(&self) -> WebsocketMetrics {
        self.metrics_tx.borrow().clone()
    }

    pub fn is_healthy(&self, max_message_age: Duration) -> bool {
        if self.connection_state() != ConnectionState::Connected {
            return false;
        }

        self.metrics()
            .last_message_at
            .map(|last_message_at| last_message_at.elapsed() <= max_message_age)
            .unwrap_or(false)
    }

    pub async fn start(&mut self) -> Result<mpsc::Receiver<BitgetWebsocketEvent>, Error> {
        if self.task.is_some() {
            return Err(Error::WebSocketError(
                "WebSocket manager 已经启动".to_string(),
            ));
        }

        let (message_tx, message_rx) =
            mpsc::channel::<BitgetWebsocketEvent>(WEBSOCKET_CHANNEL_SIZE);
        let (command_tx, command_rx) =
            mpsc::channel::<BitgetWebsocketCommand>(WEBSOCKET_CHANNEL_SIZE);
        let (stop_tx, stop_rx) = watch::channel(false);
        self.command_tx = Some(command_tx);
        self.stop_tx = Some(stop_tx);

        let context = ReconnectLoopContext {
            urls: self.urls.clone(),
            config: self.config.clone(),
            subscriptions: self.subscriptions.clone(),
            login_credentials: self.login_credentials.clone(),
            proxy_url: self.proxy_url.clone(),
            command_rx,
            message_tx,
            stop_rx,
            state_tx: self.state_tx.clone(),
            metrics_tx: self.metrics_tx.clone(),
        };
        self.task = Some(tokio::spawn(async move {
            run_reconnect_loop(context).await;
        }));

        Ok(message_rx)
    }

    pub async fn stop(&mut self) {
        self.command_tx = None;
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(true);
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
        self.state_tx.send_replace(ConnectionState::Stopped);
    }

    async fn send_command(&self, command: BitgetWebsocketCommand) -> Result<(), Error> {
        if let Some(command_tx) = &self.command_tx {
            command_tx.send(command).await.map_err(|err| {
                Error::WebSocketError(format!("发送 Bitget WebSocket manager 命令失败: {err}"))
            })?;
        }

        Ok(())
    }
}

struct ReconnectLoopContext {
    urls: Vec<String>,
    config: ReconnectConfig,
    subscriptions: Vec<BitgetWebsocketChannel>,
    login_credentials: Option<Credentials>,
    proxy_url: Option<String>,
    command_rx: mpsc::Receiver<BitgetWebsocketCommand>,
    message_tx: mpsc::Sender<BitgetWebsocketEvent>,
    stop_rx: watch::Receiver<bool>,
    state_tx: watch::Sender<ConnectionState>,
    metrics_tx: watch::Sender<WebsocketMetrics>,
}

async fn run_reconnect_loop(mut context: ReconnectLoopContext) {
    let mut attempts = 0;
    let mut current_url_idx = 0;
    let mut backoff_delay = context
        .config
        .reconnect_interval
        .min(context.config.max_backoff);

    while !*context.stop_rx.borrow() && attempts <= context.config.max_reconnect_attempts {
        context.state_tx.send_replace(if attempts == 0 {
            ConnectionState::Connecting
        } else {
            ConnectionState::Reconnecting
        });

        let url = context
            .urls
            .get(current_url_idx)
            .cloned()
            .unwrap_or_else(|| context.urls[0].clone());

        match connect_websocket(&url, context.proxy_url.as_deref()).await {
            Ok((stream, _)) => {
                context.state_tx.send_replace(ConnectionState::Connected);
                update_metrics(&context.metrics_tx, |metrics| {
                    metrics.connected_at = Some(Instant::now());
                    if metrics.connection_attempts > 0 {
                        metrics.reconnects += 1;
                    }
                    metrics.connection_attempts += 1;
                    metrics.last_error = None;
                });
                let should_reconnect = run_connected_socket(
                    stream,
                    ConnectedSocketContext {
                        subscriptions: &mut context.subscriptions,
                        command_rx: &mut context.command_rx,
                        login_credentials: context.login_credentials.as_ref(),
                        message_tx: &context.message_tx,
                        stop_rx: &mut context.stop_rx,
                        metrics_tx: &context.metrics_tx,
                        ping_interval: context.config.ping_interval,
                        message_timeout: context.config.message_timeout,
                    },
                )
                .await;
                if !should_reconnect {
                    break;
                }
                attempts += 1;
                current_url_idx = next_websocket_url_index(&context.urls, current_url_idx);
            }
            Err(err) => {
                let message = err.to_string();
                update_metrics(&context.metrics_tx, |metrics| {
                    metrics.last_error = Some(message);
                    metrics.connection_attempts += 1;
                });
                attempts += 1;
                current_url_idx = next_websocket_url_index(&context.urls, current_url_idx);
            }
        }

        if *context.stop_rx.borrow() || attempts > context.config.max_reconnect_attempts {
            break;
        }

        tokio::select! {
            _ = sleep(backoff_delay) => {}
            _ = context.stop_rx.changed() => {}
        }
        backoff_delay = next_backoff_delay(
            backoff_delay,
            context.config.backoff_factor,
            context.config.max_backoff,
        );
    }

    context.state_tx.send_replace(if *context.stop_rx.borrow() {
        ConnectionState::Stopped
    } else {
        ConnectionState::Disconnected
    });
}

struct ConnectedSocketContext<'a> {
    subscriptions: &'a mut Vec<BitgetWebsocketChannel>,
    command_rx: &'a mut mpsc::Receiver<BitgetWebsocketCommand>,
    login_credentials: Option<&'a Credentials>,
    message_tx: &'a mpsc::Sender<BitgetWebsocketEvent>,
    stop_rx: &'a mut watch::Receiver<bool>,
    metrics_tx: &'a watch::Sender<WebsocketMetrics>,
    ping_interval: Duration,
    message_timeout: Duration,
}

async fn run_connected_socket<S>(
    stream: tokio_tungstenite::WebSocketStream<S>,
    context: ConnectedSocketContext<'_>,
) -> bool
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let (mut write, mut read) = stream.split();
    if let Some(credentials) = context.login_credentials {
        let Ok(request) = login_request(credentials, current_timestamp_millis()) else {
            return true;
        };
        if write
            .send(Message::Text(request.to_string().into()))
            .await
            .is_err()
        {
            return true;
        }

        loop {
            let login_message = match timeout(context.ping_interval, read.next()).await {
                Ok(message) => message,
                Err(_) => {
                    update_metrics(context.metrics_tx, |metrics| {
                        metrics.last_error = Some(format!(
                            "Bitget WebSocket login ack 超时: {:?}",
                            context.ping_interval
                        ));
                    });
                    return true;
                }
            };

            match login_message {
                Some(Ok(Message::Text(text))) => {
                    match process_login_wait_text(
                        text.as_str(),
                        context.message_tx,
                        context.metrics_tx,
                    )
                    .await
                    {
                        LoginWaitOutcome::Authenticated => break,
                        LoginWaitOutcome::Continue => {}
                        LoginWaitOutcome::Reconnect => return true,
                        LoginWaitOutcome::Stop => return false,
                    }
                }
                Some(Ok(Message::Binary(bytes))) => {
                    let Ok(text) = std::str::from_utf8(&bytes) else {
                        continue;
                    };
                    match process_login_wait_text(text, context.message_tx, context.metrics_tx)
                        .await
                    {
                        LoginWaitOutcome::Authenticated => break,
                        LoginWaitOutcome::Continue => {}
                        LoginWaitOutcome::Reconnect => return true,
                        LoginWaitOutcome::Stop => return false,
                    }
                }
                Some(Ok(Message::Ping(payload))) => {
                    match write.send(Message::Pong(payload)).await {
                        Ok(()) => {}
                        Err(_) => return true,
                    }
                }
                Some(Ok(Message::Pong(_))) => {}
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return true,
                _ => {}
            }
        }
    }

    if !context.subscriptions.is_empty()
        && write
            .send(Message::Text(
                BitgetWebsocket::subscribe_request(context.subscriptions.as_slice()).into(),
            ))
            .await
            .is_err()
    {
        return true;
    }

    let mut ping_timer = interval(context.ping_interval);
    ping_timer.reset();
    let stale_after = context.message_timeout;
    let stale_check_interval = std::cmp::max(
        Duration::from_millis(1),
        std::cmp::min(context.ping_interval, context.message_timeout),
    );
    let mut stale_timer = interval(stale_check_interval);
    stale_timer.reset();
    let mut last_inbound_at = Instant::now();

    loop {
        tokio::select! {
            _ = context.stop_rx.changed() => {
                let _ = write.send(Message::Close(None)).await;
                return false;
            }
            command = context.command_rx.recv() => {
                let Some(command) = command else {
                    return false;
                };
                match command {
                    BitgetWebsocketCommand::Subscribe(channel) => {
                        if !context.subscriptions.contains(&channel) {
                            context.subscriptions.push(channel.clone());
                        }
                        if write
                            .send(Message::Text(
                                BitgetWebsocket::subscribe_request(std::slice::from_ref(&channel)).into(),
                            ))
                            .await
                            .is_err()
                        {
                            return true;
                        }
                    }
                    BitgetWebsocketCommand::Unsubscribe(channel) => {
                        context.subscriptions.retain(|item| item != &channel);
                        if write
                            .send(Message::Text(
                                BitgetWebsocket::unsubscribe_request(std::slice::from_ref(&channel)).into(),
                            ))
                            .await
                            .is_err()
                        {
                            return true;
                        }
                    }
                }
            }
            _ = ping_timer.tick() => {
                if write.send(Message::Text("ping".into())).await.is_err() {
                    return true;
                }
            }
            _ = stale_timer.tick() => {
                if last_inbound_at.elapsed() >= stale_after {
                    update_metrics(context.metrics_tx, |metrics| {
                        metrics.last_error = Some(format!(
                            "Bitget WebSocket 入站消息超时: {stale_after:?}"
                        ));
                    });
                    return true;
                }
            }
            message = read.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        last_inbound_at = Instant::now();
                        if forward_event(text.as_str(), context.message_tx).await.is_err() {
                            return false;
                        }
                        record_message(context.metrics_tx);
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        last_inbound_at = Instant::now();
                        let Ok(text) = std::str::from_utf8(&bytes) else {
                            continue;
                        };
                        if forward_event(text, context.message_tx).await.is_err() {
                            return false;
                        }
                        record_message(context.metrics_tx);
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        last_inbound_at = Instant::now();
                        let send_failed = write.send(Message::Pong(payload)).await.is_err();
                        if send_failed {
                            return true;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_inbound_at = Instant::now();
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return true,
                    _ => {}
                }
            }
        }
    }
}

enum LoginWaitOutcome {
    Continue,
    Authenticated,
    Reconnect,
    Stop,
}

async fn process_login_wait_text(
    text: &str,
    message_tx: &mpsc::Sender<BitgetWebsocketEvent>,
    metrics_tx: &watch::Sender<WebsocketMetrics>,
) -> LoginWaitOutcome {
    let event = match BitgetWebsocketEvent::parse(text) {
        Ok(event) => event,
        Err(err) => {
            update_metrics(metrics_tx, |metrics| {
                metrics.last_error = Some(format!("解析 Bitget WebSocket login ack 失败: {err}"));
            });
            return LoginWaitOutcome::Reconnect;
        }
    };
    let login_result = login_wait_result(&event);
    let login_failure = login_wait_failure_message(&event);
    if message_tx.send(event).await.is_err() {
        return LoginWaitOutcome::Stop;
    }
    record_message(metrics_tx);

    match login_result {
        Some(true) => LoginWaitOutcome::Authenticated,
        Some(false) => {
            update_metrics(metrics_tx, |metrics| {
                metrics.last_error = login_failure;
            });
            LoginWaitOutcome::Reconnect
        }
        None => LoginWaitOutcome::Continue,
    }
}

fn login_wait_result(event: &BitgetWebsocketEvent) -> Option<bool> {
    match event {
        BitgetWebsocketEvent::Login { code, .. } => {
            let success = matches!(code.as_deref(), Some("0") | Some("00000"));
            Some(success)
        }
        BitgetWebsocketEvent::Error { .. } => Some(false),
        _ => None,
    }
}

fn login_wait_failure_message(event: &BitgetWebsocketEvent) -> Option<String> {
    match event {
        BitgetWebsocketEvent::Login { code, msg, .. } => Some(format!(
            "Bitget WebSocket login failed: code={}, msg={}",
            code.as_deref().unwrap_or("<missing>"),
            msg.as_deref().unwrap_or("<missing>")
        )),
        BitgetWebsocketEvent::Error { code, msg, .. } => Some(format!(
            "Bitget WebSocket login error: code={}, msg={}",
            code.as_deref().unwrap_or("<missing>"),
            msg.as_deref().unwrap_or("<missing>")
        )),
        _ => None,
    }
}

fn update_metrics<F>(metrics_tx: &watch::Sender<WebsocketMetrics>, update: F)
where
    F: FnOnce(&mut WebsocketMetrics),
{
    let mut metrics = metrics_tx.borrow().clone();
    update(&mut metrics);
    metrics_tx.send_replace(metrics);
}

fn record_message(metrics_tx: &watch::Sender<WebsocketMetrics>) {
    update_metrics(metrics_tx, |metrics| {
        metrics.messages_received += 1;
        metrics.last_message_at = Some(Instant::now());
    });
}

impl BitgetWebsocketSession {
    pub async fn connect(url: &str) -> Result<Self, Error> {
        Self::connect_with_proxy(url, None).await
    }

    pub async fn connect_with_proxy(url: &str, proxy_url: Option<&str>) -> Result<Self, Error> {
        let (stream, _) = connect_websocket(url, proxy_url).await?;
        let (mut write, mut read) = stream.split();
        let (tx_in, mut rx_in) = mpsc::channel::<Message>(WEBSOCKET_CHANNEL_SIZE);
        let (tx_out, rx_out) = mpsc::channel::<BitgetWebsocketEvent>(WEBSOCKET_CHANNEL_SIZE);
        let ping_tx = tx_in.clone();

        tokio::spawn(async move {
            while let Some(message) = rx_in.recv().await {
                if write.send(message).await.is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        let forward_failed = forward_event(text.as_str(), &tx_out).await.is_err();
                        if forward_failed {
                            break;
                        }
                    }
                    Ok(Message::Binary(bytes)) => {
                        let Ok(text) = std::str::from_utf8(&bytes) else {
                            continue;
                        };
                        if forward_event(text, &tx_out).await.is_err() {
                            break;
                        }
                    }
                    Ok(Message::Ping(payload)) => {
                        let send_failed = ping_tx.send(Message::Pong(payload)).await.is_err();
                        if send_failed {
                            break;
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => break,
                    _ => {}
                }
            }
        });

        Ok(Self {
            tx: tx_in,
            rx: rx_out,
        })
    }

    pub async fn recv_event(&mut self) -> Option<BitgetWebsocketEvent> {
        self.rx.recv().await
    }

    pub async fn ping(&self) -> Result<(), Error> {
        self.tx
            .send(Message::Text("ping".into()))
            .await
            .map_err(|err| Error::WebSocketError(format!("发送 ping 失败: {err}")))
    }

    pub async fn login(&self, request: Value) -> Result<(), Error> {
        self.send_json(request).await
    }

    pub async fn subscribe(&self, channels: &[BitgetWebsocketChannel]) -> Result<(), Error> {
        self.send_text(BitgetWebsocket::subscribe_request(channels))
            .await
    }

    pub async fn unsubscribe(&self, channels: &[BitgetWebsocketChannel]) -> Result<(), Error> {
        self.send_text(BitgetWebsocket::unsubscribe_request(channels))
            .await
    }

    pub async fn close(&self) -> Result<(), Error> {
        self.tx
            .send(Message::Close(None))
            .await
            .map_err(|err| Error::WebSocketError(format!("发送关闭消息失败: {err}")))
    }

    async fn send_json(&self, value: Value) -> Result<(), Error> {
        self.send_text(value.to_string()).await
    }

    async fn send_text(&self, text: String) -> Result<(), Error> {
        self.tx
            .send(Message::Text(text.into()))
            .await
            .map_err(|err| Error::WebSocketError(format!("发送 WebSocket 消息失败: {err}")))
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

fn build_websocket_url_pool(primary: String) -> Vec<String> {
    let mut urls = Vec::new();
    push_websocket_url_candidate(&mut urls, &primary);

    if let Ok(extra_urls) = env::var("BITGET_WS_FALLBACKS") {
        for item in extra_urls.split(',') {
            push_websocket_url_candidate(&mut urls, item);
        }
    }

    if urls.is_empty() {
        urls.push(primary);
    }

    urls
}

fn push_websocket_url_candidate(urls: &mut Vec<String>, candidate: &str) {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return;
    }

    let normalized = match url::Url::parse(trimmed) {
        Ok(url) if matches!(url.scheme(), "ws" | "wss") && url.host_str().is_some() => {
            url.to_string()
        }
        Ok(_) => return,
        Err(_) => trimmed.to_string(),
    };

    if !urls.contains(&normalized) {
        urls.push(normalized);
    }
}

fn next_websocket_url_index(urls: &[String], current_url_idx: usize) -> usize {
    if urls.len() <= 1 {
        current_url_idx
    } else {
        (current_url_idx + 1) % urls.len()
    }
}

fn next_backoff_delay(current: Duration, backoff_factor: f64, max_backoff: Duration) -> Duration {
    if current >= max_backoff {
        return max_backoff;
    }
    if !backoff_factor.is_finite() || backoff_factor <= 1.0 {
        return current.min(max_backoff);
    }

    let scaled = current.as_secs_f64() * backoff_factor;
    if !scaled.is_finite() {
        return max_backoff;
    }

    Duration::from_secs_f64(scaled.min(max_backoff.as_secs_f64()))
}

async fn connect_websocket(
    url: &str,
    proxy_url: Option<&str>,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), Error> {
    if let Some(proxy_addr) = proxy_url.and_then(socks5_proxy_addr) {
        return connect_websocket_via_socks5(url, &proxy_addr).await;
    }

    connect_async(url)
        .await
        .map_err(|err| Error::WebSocketError(format!("连接失败: {err}")))
}

async fn connect_websocket_via_socks5(
    url: &str,
    proxy_addr: &str,
) -> Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, Response), Error> {
    let parsed = url::Url::parse(url)
        .map_err(|err| Error::WebSocketError(format!("WebSocket URL 无效: {err}")))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| Error::WebSocketError("WebSocket URL 缺少 host".to_string()))?;
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| Error::WebSocketError("WebSocket URL 缺少 port".to_string()))?;

    let mut stream = TcpStream::connect(proxy_addr)
        .await
        .map_err(|err| Error::WebSocketError(format!("连接 SOCKS5 代理失败: {err}")))?;
    socks5_connect(&mut stream, host, port).await?;

    client_async_tls_with_config(url, stream, None, None)
        .await
        .map_err(|err| Error::WebSocketError(format!("代理 WebSocket 握手失败: {err}")))
}

async fn socks5_connect(stream: &mut TcpStream, host: &str, port: u16) -> Result<(), Error> {
    let host_bytes = host.as_bytes();
    if host_bytes.len() > u8::MAX as usize {
        return Err(Error::WebSocketError(
            "SOCKS5 目标 host 长度超过 255 字节".to_string(),
        ));
    }

    stream
        .write_all(&[0x05, 0x01, 0x00])
        .await
        .map_err(|err| Error::WebSocketError(format!("发送 SOCKS5 greeting 失败: {err}")))?;
    let mut greeting = [0_u8; 2];
    stream
        .read_exact(&mut greeting)
        .await
        .map_err(|err| Error::WebSocketError(format!("读取 SOCKS5 greeting 失败: {err}")))?;
    if greeting != [0x05, 0x00] {
        return Err(Error::WebSocketError(format!(
            "SOCKS5 代理不支持 no-auth: {greeting:?}"
        )));
    }

    let mut request = Vec::with_capacity(7 + host_bytes.len());
    request.extend_from_slice(&[0x05, 0x01, 0x00, 0x03, host_bytes.len() as u8]);
    request.extend_from_slice(host_bytes);
    request.extend_from_slice(&port.to_be_bytes());
    stream
        .write_all(&request)
        .await
        .map_err(|err| Error::WebSocketError(format!("发送 SOCKS5 connect 请求失败: {err}")))?;

    let mut response = [0_u8; 4];
    stream
        .read_exact(&mut response)
        .await
        .map_err(|err| Error::WebSocketError(format!("读取 SOCKS5 响应失败: {err}")))?;
    if response[0] != 0x05 || response[1] != 0x00 {
        return Err(Error::WebSocketError(format!(
            "SOCKS5 connect 失败: {response:?}"
        )));
    }

    match response[3] {
        0x01 => read_exact_discard(stream, 4).await?,
        0x03 => {
            let mut len = [0_u8; 1];
            stream
                .read_exact(&mut len)
                .await
                .map_err(|err| Error::WebSocketError(format!("读取 SOCKS5 地址长度失败: {err}")))?;
            read_exact_discard(stream, usize::from(len[0])).await?;
        }
        0x04 => read_exact_discard(stream, 16).await?,
        other => {
            return Err(Error::WebSocketError(format!(
                "SOCKS5 响应地址类型不支持: {other}"
            )));
        }
    }
    read_exact_discard(stream, 2).await?;

    Ok(())
}

async fn read_exact_discard(stream: &mut TcpStream, len: usize) -> Result<(), Error> {
    let mut buffer = vec![0_u8; len];
    stream
        .read_exact(&mut buffer)
        .await
        .map_err(|err| Error::WebSocketError(format!("读取 SOCKS5 响应字段失败: {err}")))?;
    Ok(())
}

fn socks5_proxy_addr(proxy_url: &str) -> Option<String> {
    let trimmed = proxy_url.trim();
    let rest = trimmed
        .strip_prefix("socks5h://")
        .or_else(|| trimmed.strip_prefix("socks5://"))?;
    let authority = rest.split('/').next().unwrap_or(rest);
    if authority.is_empty() {
        None
    } else {
        Some(authority.to_string())
    }
}

async fn forward_event(text: &str, tx: &mpsc::Sender<BitgetWebsocketEvent>) -> Result<(), ()> {
    match BitgetWebsocketEvent::parse(text) {
        Ok(event) => tx.send(event).await.map_err(|_| ()),
        Err(_) => Ok(()),
    }
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
