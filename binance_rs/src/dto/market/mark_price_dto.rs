use super::BinanceWireDecimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Binance USD-M `/fapi/v1/premiumIndex` 的单标的标记价响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceUsdmMarkPrice {
    /// 请求和下单共同使用的 provider symbol。
    pub symbol: String,
    /// Provider 原始标记价，保留字符串或 JSON number 的无损表达。
    pub mark_price: BinanceWireDecimal,
    /// Provider 生成该价格的 Unix 毫秒时间戳。
    pub time: u64,
    /// 当前门禁不消费但需要保留的 provider 扩展字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
