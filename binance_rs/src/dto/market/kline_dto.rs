use super::instrument_dto::BinanceWireDecimal;
use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::fmt;

/// Binance USD-M `/fapi/v1/klines` 的无损单行 wire DTO。
///
/// 标准 12 个数组位置全部必需；价格与成交量保留 provider decimal 表示，
/// 尾部新增位置保存在 `extra`。该 DTO 不根据 close time 推断 K 线是否最终确认。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinanceUsdmKline {
    /// K 线开盘时间，Unix 毫秒。
    pub open_time: u64,
    /// 开盘价的 provider decimal 表示。
    pub open: BinanceWireDecimal,
    /// 最高价的 provider decimal 表示。
    pub high: BinanceWireDecimal,
    /// 最低价的 provider decimal 表示。
    pub low: BinanceWireDecimal,
    /// 收盘价的 provider decimal 表示。
    pub close: BinanceWireDecimal,
    /// 标的资产成交量的 provider decimal 表示。
    pub base_volume: BinanceWireDecimal,
    /// K 线收盘时间，Unix 毫秒；该值本身不等于 finality 证据。
    pub close_time: u64,
    /// 报价资产成交量的 provider decimal 表示。
    pub quote_volume: BinanceWireDecimal,
    /// 该 K 线包含的成交笔数。
    pub trade_count: u64,
    /// 主动买入标的资产成交量。
    pub taker_buy_base_volume: BinanceWireDecimal,
    /// 主动买入报价资产成交量。
    pub taker_buy_quote_volume: BinanceWireDecimal,
    /// Binance 标记为 ignore 的标准位置；SDK只保留，不赋予业务语义。
    pub ignore: Value,
    /// Binance 未来追加在标准 12 个位置后的未知字段。
    pub extra: Vec<Value>,
}

impl<'de> Deserialize<'de> for BinanceUsdmKline {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(BinanceUsdmKlineVisitor)
    }
}

/// 按 Binance 固定位置解码一行，同时允许尾部出现未来扩展字段。
struct BinanceUsdmKlineVisitor;

impl<'de> Visitor<'de> for BinanceUsdmKlineVisitor {
    type Value = BinanceUsdmKline;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Binance USD-M Kline array with at least 12 fields")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let open_time = required(&mut sequence, 0, "open time")?;
        let open = required(&mut sequence, 1, "open")?;
        let high = required(&mut sequence, 2, "high")?;
        let low = required(&mut sequence, 3, "low")?;
        let close = required(&mut sequence, 4, "close")?;
        let base_volume = required(&mut sequence, 5, "base volume")?;
        let close_time = required(&mut sequence, 6, "close time")?;
        let quote_volume = required(&mut sequence, 7, "quote volume")?;
        let trade_count = required(&mut sequence, 8, "trade count")?;
        let taker_buy_base_volume = required(&mut sequence, 9, "taker buy base volume")?;
        let taker_buy_quote_volume = required(&mut sequence, 10, "taker buy quote volume")?;
        let ignore = required(&mut sequence, 11, "ignore")?;
        let mut extra = Vec::new();
        while let Some(value) = sequence.next_element()? {
            extra.push(value);
        }

        Ok(BinanceUsdmKline {
            open_time,
            open,
            high,
            low,
            close,
            base_volume,
            close_time,
            quote_volume,
            trade_count,
            taker_buy_base_volume,
            taker_buy_quote_volume,
            ignore,
            extra,
        })
    }
}

/// 读取固定位置并生成包含位置/字段名的协议错误，便于 evidence 定位损坏行。
fn required<'de, A, T>(sequence: &mut A, index: usize, field: &'static str) -> Result<T, A::Error>
where
    A: SeqAccess<'de>,
    T: Deserialize<'de>,
{
    sequence
        .next_element()?
        .ok_or_else(|| de::Error::custom(format!("missing Kline field {index} ({field})")))
}
