use crate::error::Error;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum BinanceWebsocketEvent {
    ListenKeyExpired(ListenKeyExpiredEvent),
    MarginCall(MarginCallEvent),
    OrderTradeUpdate(OrderTradeUpdateEvent),
    TradeLite(TradeLiteEvent),
    AccountUpdate(AccountUpdateEvent),
    AccountConfigUpdate(AccountConfigUpdateEvent),
    StrategyUpdate(StrategyUpdateEvent),
    GridUpdate(GridUpdateEvent),
    ConditionalOrderTriggerReject(ConditionalOrderTriggerRejectEvent),
    AlgoUpdate(Box<AlgoUpdateEvent>),
    Raw(Value),
}

impl BinanceWebsocketEvent {
    pub fn parse(value: Value) -> Result<Self, Error> {
        let typed_payload = value
            .get("data")
            .filter(|data| data.get("e").is_some())
            .cloned()
            .unwrap_or_else(|| value.clone());
        let event = typed_payload.get("e").and_then(Value::as_str);

        match event {
            Some("listenKeyExpired") => {
                serde_json::from_value::<ListenKeyExpiredEvent>(typed_payload)
                    .map(Self::ListenKeyExpired)
                    .map_err(Error::JsonError)
            }
            Some("MARGIN_CALL") => serde_json::from_value::<MarginCallEvent>(typed_payload)
                .map(Self::MarginCall)
                .map_err(Error::JsonError),
            Some("ORDER_TRADE_UPDATE") => {
                serde_json::from_value::<OrderTradeUpdateEvent>(typed_payload)
                    .map(Self::OrderTradeUpdate)
                    .map_err(Error::JsonError)
            }
            Some("TRADE_LITE") => serde_json::from_value::<TradeLiteEvent>(typed_payload)
                .map(Self::TradeLite)
                .map_err(Error::JsonError),
            Some("ACCOUNT_UPDATE") => serde_json::from_value::<AccountUpdateEvent>(typed_payload)
                .map(Self::AccountUpdate)
                .map_err(Error::JsonError),
            Some("ACCOUNT_CONFIG_UPDATE") => {
                serde_json::from_value::<AccountConfigUpdateEvent>(typed_payload)
                    .map(Self::AccountConfigUpdate)
                    .map_err(Error::JsonError)
            }
            Some("STRATEGY_UPDATE") => serde_json::from_value::<StrategyUpdateEvent>(typed_payload)
                .map(Self::StrategyUpdate)
                .map_err(Error::JsonError),
            Some("GRID_UPDATE") => serde_json::from_value::<GridUpdateEvent>(typed_payload)
                .map(Self::GridUpdate)
                .map_err(Error::JsonError),
            Some("CONDITIONAL_ORDER_TRIGGER_REJECT") => {
                serde_json::from_value::<ConditionalOrderTriggerRejectEvent>(typed_payload)
                    .map(Self::ConditionalOrderTriggerReject)
                    .map_err(Error::JsonError)
            }
            Some("ALGO_UPDATE") => serde_json::from_value::<AlgoUpdateEvent>(typed_payload)
                .map(Box::new)
                .map(Self::AlgoUpdate)
                .map_err(Error::JsonError),
            _ => Ok(Self::Raw(value)),
        }
    }
}

fn deserialize_u64_from_string_or_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Number(number) => number
            .as_u64()
            .ok_or_else(|| serde::de::Error::custom("expected unsigned integer")),
        Value::String(text) => text
            .parse::<u64>()
            .map_err(|err| serde::de::Error::custom(format!("expected unsigned integer: {err}"))),
        _ => Err(serde::de::Error::custom(
            "expected unsigned integer as number or string",
        )),
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ListenKeyExpiredEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(rename = "listenKey")]
    pub listen_key: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MarginCallEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(rename = "cw", default)]
    pub cross_wallet_balance: Option<String>,
    #[serde(rename = "p", default)]
    pub positions: Vec<MarginCallPosition>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MarginCallPosition {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "ps")]
    pub position_side: String,
    #[serde(rename = "pa")]
    pub position_amount: String,
    #[serde(rename = "mt")]
    pub margin_type: String,
    #[serde(rename = "iw", default)]
    pub isolated_wallet: Option<String>,
    #[serde(rename = "mp")]
    pub mark_price: String,
    #[serde(rename = "up")]
    pub unrealized_pnl: String,
    #[serde(rename = "mm")]
    pub maintenance_margin_required: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OrderTradeUpdateEvent {
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "T")]
    pub transaction_time: u64,
    #[serde(rename = "o")]
    pub order: OrderTradeUpdate,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OrderTradeUpdate {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "c")]
    pub client_order_id: String,
    #[serde(rename = "S")]
    pub side: String,
    #[serde(rename = "o")]
    pub order_type: String,
    #[serde(rename = "x")]
    pub execution_type: String,
    #[serde(rename = "X")]
    pub status: String,
    #[serde(rename = "i")]
    pub order_id: u64,
    #[serde(rename = "q")]
    pub original_quantity: String,
    #[serde(rename = "p")]
    pub original_price: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TradeLiteEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(
        rename = "T",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub transaction_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "q")]
    pub original_quantity: String,
    #[serde(rename = "p")]
    pub original_price: String,
    #[serde(rename = "m")]
    pub is_maker: bool,
    #[serde(rename = "c")]
    pub client_order_id: String,
    #[serde(rename = "S")]
    pub side: String,
    #[serde(rename = "L")]
    pub last_filled_price: String,
    #[serde(rename = "l")]
    pub last_filled_quantity: String,
    #[serde(
        rename = "t",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub trade_id: u64,
    #[serde(
        rename = "i",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub order_id: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountUpdateEvent {
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "T")]
    pub transaction_time: u64,
    #[serde(rename = "a")]
    pub data: AccountUpdateData,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountUpdateData {
    #[serde(rename = "m")]
    pub reason: String,
    #[serde(rename = "B", default)]
    pub balances: Vec<AccountUpdateBalance>,
    #[serde(rename = "P", default)]
    pub positions: Vec<AccountUpdatePosition>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountUpdateBalance {
    #[serde(rename = "a")]
    pub asset: String,
    #[serde(rename = "wb")]
    pub wallet_balance: String,
    #[serde(rename = "cw")]
    pub cross_wallet_balance: String,
    #[serde(rename = "bc")]
    pub balance_change: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountUpdatePosition {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "pa")]
    pub position_amount: String,
    #[serde(rename = "ep")]
    pub entry_price: String,
    #[serde(rename = "bep")]
    pub breakeven_price: String,
    #[serde(rename = "cr")]
    pub accumulated_realized: String,
    #[serde(rename = "up")]
    pub unrealized_pnl: String,
    #[serde(rename = "mt")]
    pub margin_type: String,
    #[serde(rename = "iw", default)]
    pub isolated_wallet: String,
    #[serde(rename = "ps")]
    pub position_side: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountConfigUpdateEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(
        rename = "T",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub transaction_time: u64,
    #[serde(rename = "ac", default)]
    pub symbol_config: Option<AccountConfigSymbolUpdate>,
    #[serde(rename = "ai", default)]
    pub user_config: Option<AccountConfigUserUpdate>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountConfigSymbolUpdate {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(
        rename = "l",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub leverage: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AccountConfigUserUpdate {
    #[serde(rename = "j")]
    pub multi_assets_margin_mode: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct StrategyUpdateEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(
        rename = "T",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub transaction_time: u64,
    #[serde(rename = "su")]
    pub update: StrategyUpdate,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct StrategyUpdate {
    #[serde(
        rename = "si",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub strategy_id: u64,
    #[serde(rename = "st")]
    pub strategy_type: String,
    #[serde(rename = "ss")]
    pub strategy_status: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(
        rename = "ut",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub update_time: u64,
    #[serde(
        rename = "c",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub opcode: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct GridUpdateEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(
        rename = "T",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub transaction_time: u64,
    #[serde(rename = "gu")]
    pub update: GridUpdate,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct GridUpdate {
    #[serde(
        rename = "si",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub strategy_id: u64,
    #[serde(rename = "st")]
    pub strategy_type: String,
    #[serde(rename = "ss")]
    pub strategy_status: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "r")]
    pub realized_pnl: String,
    #[serde(rename = "up")]
    pub unmatched_average_price: String,
    #[serde(rename = "uq")]
    pub unmatched_quantity: String,
    #[serde(rename = "uf")]
    pub unmatched_fee: String,
    #[serde(rename = "mp")]
    pub matched_pnl: String,
    #[serde(
        rename = "ut",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub update_time: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ConditionalOrderTriggerRejectEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(
        rename = "T",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub message_send_time: u64,
    #[serde(rename = "or")]
    pub order: ConditionalOrderTriggerRejectOrder,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ConditionalOrderTriggerRejectOrder {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(
        rename = "i",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub order_id: u64,
    #[serde(rename = "r")]
    pub reject_reason: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AlgoUpdateEvent {
    #[serde(
        rename = "E",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub event_time: u64,
    #[serde(
        rename = "T",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub transaction_time: u64,
    #[serde(rename = "o")]
    pub order: AlgoOrderUpdate,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct AlgoOrderUpdate {
    #[serde(rename = "caid")]
    pub client_algo_id: String,
    #[serde(
        rename = "aid",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub algo_id: u64,
    #[serde(rename = "at")]
    pub algo_type: String,
    #[serde(rename = "o")]
    pub order_type: String,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "S")]
    pub side: String,
    #[serde(rename = "ps")]
    pub position_side: String,
    #[serde(rename = "f")]
    pub time_in_force: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "X")]
    pub algo_status: String,
    #[serde(rename = "ai")]
    pub order_id: String,
    #[serde(rename = "ap", default)]
    pub average_price: Option<String>,
    #[serde(rename = "aq", default)]
    pub executed_quantity: Option<String>,
    #[serde(rename = "act", default)]
    pub actual_order_type: Option<String>,
    #[serde(rename = "tp")]
    pub trigger_price: String,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "V")]
    pub self_trade_prevention_mode: String,
    #[serde(rename = "wt")]
    pub working_type: String,
    #[serde(rename = "pm")]
    pub price_match_mode: String,
    #[serde(rename = "cp")]
    pub close_position: bool,
    #[serde(rename = "pP")]
    pub price_protection: bool,
    #[serde(rename = "R")]
    pub reduce_only: bool,
    #[serde(
        rename = "tt",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub trigger_time: u64,
    #[serde(
        rename = "gtd",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub good_till_date: u64,
    #[serde(rename = "rm", default)]
    pub reject_reason: Option<String>,
}
