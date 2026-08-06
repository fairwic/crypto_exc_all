use crate::adapters::ExchangeClient;
use crate::error::Result;
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub side: Option<String>,
    pub size: String,
    pub entry_price: Option<String>,
    pub mark_price: Option<String>,
    pub unrealized_pnl: Option<String>,
    pub leverage: Option<String>,
    pub margin_mode: Option<String>,
    pub liquidation_price: Option<String>,
    pub raw: Value,
}

/// 带 provider 更新时间的持仓读取结果；旧 `Position` 合同保持不变。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourcedPosition {
    /// 既有 canonical 持仓事实。
    pub position: Position,
    /// 交易所报告的持仓更新时间（Unix 毫秒）。
    pub source_updated_at_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PositionHistoryQuery {
    pub instrument: Option<Instrument>,
    pub instrument_type: Option<String>,
    pub margin_mode: Option<String>,
    pub close_type: Option<String>,
    pub position_id: Option<String>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub limit: Option<u32>,
}

impl PositionHistoryQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_instrument(instrument: Instrument) -> Self {
        Self::new().with_instrument(instrument)
    }

    pub fn with_instrument(mut self, value: Instrument) -> Self {
        self.instrument = Some(value);
        self
    }

    pub fn with_instrument_type(mut self, value: impl Into<String>) -> Self {
        self.instrument_type = Some(value.into());
        self
    }

    pub fn with_margin_mode(mut self, value: impl Into<String>) -> Self {
        self.margin_mode = Some(value.into());
        self
    }

    pub fn with_close_type(mut self, value: impl Into<String>) -> Self {
        self.close_type = Some(value.into());
        self
    }

    pub fn with_position_id(mut self, value: impl Into<String>) -> Self {
        self.position_id = Some(value.into());
        self
    }

    pub fn with_after(mut self, value: impl Into<String>) -> Self {
        self.after = Some(value.into());
        self
    }

    pub fn with_before(mut self, value: impl Into<String>) -> Self {
        self.before = Some(value.into());
        self
    }

    pub fn with_limit(mut self, value: u32) -> Self {
        self.limit = Some(value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PositionHistory {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub position_id: Option<String>,
    pub side: Option<String>,
    pub direction: Option<String>,
    pub leverage: Option<String>,
    pub margin_mode: Option<String>,
    pub open_avg_price: Option<String>,
    pub close_avg_price: Option<String>,
    pub open_max_position: Option<String>,
    pub close_total_position: Option<String>,
    pub realized_pnl: Option<String>,
    pub pnl: Option<String>,
    pub pnl_ratio: Option<String>,
    pub fee: Option<String>,
    pub funding_fee: Option<String>,
    pub liquidation_penalty: Option<String>,
    pub close_type: Option<String>,
    pub open_time: Option<u64>,
    pub close_time: Option<u64>,
    pub raw: Value,
}

pub struct PositionFacade<'a> {
    pub(crate) client: &'a ExchangeClient,
}

impl<'a> PositionFacade<'a> {
    pub(crate) fn new(client: &'a ExchangeClient) -> Self {
        Self { client }
    }

    pub async fn list(&self, instrument: Option<&Instrument>) -> Result<Vec<Position>> {
        self.client.positions(instrument).await
    }

    /// 读取带 provider 更新时间的 OKX/Binance 持仓，供 snapshot/stream 因果合并。
    pub async fn sourced_list(
        &self,
        instrument: Option<&Instrument>,
    ) -> Result<Vec<SourcedPosition>> {
        self.client.sourced_positions(instrument).await
    }

    pub async fn history(&self, query: PositionHistoryQuery) -> Result<Vec<PositionHistory>> {
        self.client.position_history(query).await
    }
}
