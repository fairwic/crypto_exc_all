use crate::{Error, ExchangeId, Result};
use serde_json::Value;

#[cfg(feature = "binance")]
mod binance;
#[cfg(feature = "okx")]
mod okx;
mod session;

pub use session::{
    PrivateAccountStreamFacade, PrivateAccountStreamKeepalive, PrivateAccountStreamSession,
};

/// 私有流中的余额变更；数值保持交易所文本，精确 Decimal 转换由 Account Gateway 完成。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateBalanceStreamChange {
    pub asset: String,
    pub total: String,
    /// Binance USDⓈ-M `ACCOUNT_UPDATE` 不提供 REST `availableBalance`，此时必须保持缺失。
    pub available: Option<String>,
    pub source_updated_at_ms: u64,
}

/// 私有流中的持仓变更；零数量代表该 canonical 持仓已关闭。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivatePositionStreamChange {
    pub exchange_symbol: String,
    pub side: Option<String>,
    pub size: String,
    pub entry_price: Option<String>,
    pub mark_price: Option<String>,
    pub unrealized_pnl: Option<String>,
    pub leverage: Option<String>,
    pub margin_mode: Option<String>,
    pub liquidation_price: Option<String>,
    pub source_updated_at_ms: u64,
}

/// 私有流中的订单变更；Account Gateway 根据 provider 状态判定 active/terminal。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateOrderStreamChange {
    pub exchange_symbol: String,
    pub order_id: String,
    pub client_order_id: Option<String>,
    pub side: Option<String>,
    pub order_type: Option<String>,
    pub price: Option<String>,
    pub size: Option<String>,
    pub filled_size: Option<String>,
    pub status: String,
    pub created_at_ms: Option<u64>,
    pub source_updated_at_ms: u64,
}

/// SDK 只表达交易所协议事实，不在此层决定 Account 恢复或执行门禁。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrivateAccountStreamChange {
    Balance(PrivateBalanceStreamChange),
    Position(PrivatePositionStreamChange),
    Order(PrivateOrderStreamChange),
}

/// SDK 从单个 provider frame 拆出的可比较事实；业务层仍负责账户 scope 与 payload hash。
#[derive(Debug, Clone, PartialEq)]
pub struct PrivateAccountStreamRecord {
    pub exchange: ExchangeId,
    pub provider_event_time_ms: u64,
    pub provider_transaction_time_ms: Option<u64>,
    pub event_identity: String,
    pub change: PrivateAccountStreamChange,
    /// 仅用于 Gateway 计算去重 hash；不得越过 Gateway 进入 Domain。
    pub raw_payload: Value,
}

/// 私有流 frame 可以是账户事实、凭证流失效或无状态控制响应。
#[derive(Debug, Clone, PartialEq)]
pub enum PrivateAccountStreamFrame {
    Records(Vec<PrivateAccountStreamRecord>),
    Expired {
        exchange: ExchangeId,
        provider_event_time_ms: u64,
    },
    Control {
        exchange: ExchangeId,
        event: String,
    },
}

/// 复用交易所 SDK 的协议类型，把 OKX/Binance 私有 frame 收敛为 A4 最小事件合同。
pub fn parse_private_account_stream_frame(
    exchange: ExchangeId,
    payload: Value,
) -> Result<PrivateAccountStreamFrame> {
    match exchange {
        #[cfg(feature = "okx")]
        ExchangeId::Okx => okx::parse(payload),
        #[cfg(feature = "binance")]
        ExchangeId::Binance => binance::parse(payload),
        _ => Err(Error::Unsupported {
            exchange,
            capability: "private_account_stream",
        }),
    }
}

#[cfg(any(feature = "okx", feature = "binance"))]
pub(super) fn number_or_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.trim().is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

/// SDK optional 文本只去除空值，不改变 provider 大小写或数值格式。
#[cfg(feature = "binance")]
pub(super) fn nonempty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

#[cfg(any(feature = "okx", feature = "binance"))]
pub(super) fn adapter_error(exchange: ExchangeId, message: &'static str) -> Error {
    Error::Adapter {
        exchange,
        message: message.to_owned(),
    }
}
