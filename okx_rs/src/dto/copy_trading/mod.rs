use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OkxPublicLeadTraderPage {
    #[serde(rename = "dataVer")]
    pub data_version: String,
    pub ranks: Vec<OkxPublicLeadTraderRank>,
    #[serde(rename = "totalPage")]
    pub total_page: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// OKX 公共带单交易员榜单的无损 provider DTO。
///
/// 金额、比例和时间保持交易所原始文本；调用方不得把榜单更新当成订单或成交事件。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OkxPublicLeadTraderRank {
    #[serde(rename = "uniqueCode")]
    pub unique_code: String,
    #[serde(rename = "nickName")]
    pub nickname: String,
    #[serde(rename = "portLink", default)]
    pub avatar_url: String,
    #[serde(rename = "copyState", default)]
    pub copy_state: String,
    #[serde(rename = "copyTraderNum", default)]
    pub copy_trader_count: String,
    #[serde(rename = "maxCopyTraderNum", default)]
    pub maximum_copy_trader_count: String,
    #[serde(rename = "accCopyTraderNum", default)]
    pub accumulated_copy_trader_count: String,
    #[serde(default)]
    pub ccy: String,
    #[serde(default)]
    pub pnl: String,
    #[serde(rename = "pnlRatio", default)]
    pub pnl_ratio: String,
    #[serde(default)]
    pub aum: String,
    #[serde(rename = "winRatio", default)]
    pub win_ratio: String,
    #[serde(rename = "leadDays", default)]
    pub lead_days: String,
    #[serde(rename = "traderInsts", default)]
    pub trader_instruments: Vec<String>,
    #[serde(rename = "pnlRatios", default)]
    pub pnl_ratios: Vec<OkxPublicLeadTraderPnlRatio>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OkxPublicLeadTraderPnlRatio {
    #[serde(rename = "beginTs")]
    pub begin_timestamp_ms: String,
    #[serde(rename = "pnlRatio")]
    pub pnl_ratio: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
