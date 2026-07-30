use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// OKX 公共 `SWAP` 产品规格的无损 provider DTO。
///
/// 已知的十进制与时间字段保持交易所原始文本，SDK 不在缺少 Market
/// 业务上下文时执行精度换算；新增 provider 字段由 `extra` 保留，避免升级期间静默丢失规则。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OkxPublicInstrument {
    /// 产品类型；该 capability 的成功响应应为 OKX 原始值 `SWAP`。
    #[serde(rename = "instType")]
    pub instrument_type: String,
    /// OKX 产品唯一标识，例如 `BTC-USDT-SWAP`。
    #[serde(rename = "instId")]
    pub instrument_id: String,
    /// 标的指数，例如 `BTC-USDT`；空字符串表示 provider 未声明。
    #[serde(rename = "uly")]
    pub underlying: String,
    /// 产品家族，例如 `BTC-USDT`，用于关联同一标的的合约规格。
    #[serde(rename = "instFamily")]
    pub instrument_family: String,
    /// 交易币种；衍生品响应可能以空字符串表示不适用。
    #[serde(rename = "baseCcy")]
    pub base_currency: String,
    /// 计价币种；衍生品响应可能以空字符串表示不适用。
    #[serde(rename = "quoteCcy")]
    pub quote_currency: String,
    /// 结算币种，例如 `USDT`。
    #[serde(rename = "settleCcy")]
    pub settlement_currency: String,
    /// 单张合约面值的原始十进制文本，禁止经 `f64` 往返。
    #[serde(rename = "ctVal")]
    pub contract_value: String,
    /// 合约乘数的原始十进制文本；空字符串表示 provider 未声明。
    #[serde(rename = "ctMult")]
    pub contract_multiplier: String,
    /// 合约面值币种，用于解释 `ctVal` 的计价单位。
    #[serde(rename = "ctValCcy")]
    pub contract_value_currency: String,
    /// 期权类型；SWAP 通常为空字符串，但仍保留 provider 原值。
    #[serde(rename = "optType")]
    pub option_type: String,
    /// 行权价格原始十进制文本；SWAP 通常为空字符串。
    #[serde(rename = "stk")]
    pub strike_price: String,
    /// 上线时间，OKX Unix 毫秒时间戳文本。
    #[serde(rename = "listTime")]
    pub list_time_ms: String,
    /// 集合竞价结束时间，OKX Unix 毫秒时间戳文本；不适用时为空字符串。
    #[serde(rename = "auctionEndTime", default)]
    pub auction_end_time_ms: String,
    /// 到期时间，OKX Unix 毫秒时间戳文本；永续合约通常为空字符串。
    #[serde(rename = "expTime")]
    pub expiry_time_ms: String,
    /// 最大杠杆倍数的原始十进制文本。
    pub lever: String,
    /// 最小价格变动单位的原始十进制文本。
    #[serde(rename = "tickSz")]
    pub tick_size: String,
    /// 下单数量步长的原始十进制文本，单位为合约张数。
    #[serde(rename = "lotSz")]
    pub lot_size: String,
    /// 最小下单数量的原始十进制文本，单位为合约张数。
    #[serde(rename = "minSz")]
    pub minimum_size: String,
    /// 合约类型，例如 `linear` 或 `inverse`。
    #[serde(rename = "ctType")]
    pub contract_type: String,
    /// 合约别名；永续合约通常为空字符串。
    pub alias: String,
    /// 产品状态，例如 `live`、`suspend` 或 `preopen`。
    pub state: String,
    /// 产品规则类型；保留 OKX 原值以便 Market 选择版本化 source profile。
    #[serde(rename = "ruleType")]
    pub rule_type: String,
    /// 产品分类；`1` 表示加密货币，其他值由调用方按业务策略解释。
    #[serde(rename = "instCategory", default)]
    pub instrument_category: String,
    /// 限价单最大数量的原始十进制文本，单位为合约张数。
    #[serde(rename = "maxLmtSz")]
    pub maximum_limit_size: String,
    /// 市价单最大数量的原始十进制文本，单位为合约张数。
    #[serde(rename = "maxMktSz")]
    pub maximum_market_size: String,
    /// 时间加权订单最大数量的原始十进制文本，单位为合约张数。
    #[serde(rename = "maxTwapSz")]
    pub maximum_twap_size: String,
    /// 冰山订单最大数量的原始十进制文本，单位为合约张数。
    #[serde(rename = "maxIcebergSz")]
    pub maximum_iceberg_size: String,
    /// 计划委托最大数量的原始十进制文本，单位为合约张数。
    #[serde(rename = "maxTriggerSz")]
    pub maximum_trigger_size: String,
    /// 止盈止损订单最大数量的原始十进制文本，单位为合约张数。
    #[serde(rename = "maxStopSz")]
    pub maximum_stop_size: String,
    /// OKX 后续新增但当前 SDK 尚未命名的 provider 字段。
    ///
    /// 值使用启用 `arbitrary_precision` 的 `serde_json::Value`，未知数字不会先转成 `f64`。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
