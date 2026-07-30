use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::collections::BTreeMap;

/// Binance JSON 中以字符串或 number token 表达的十进制数。
///
/// SDK 保留 provider 的 wire 表示，不在协议边界引入 `f64` 或业务量化；
/// Market adapter 后续依据 InstrumentRules 转换为 canonical Decimal。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BinanceWireDecimal {
    /// Binance 以 JSON 字符串返回的十进制文本，可包含科学计数法。
    Text(String),
    /// Binance 以 JSON number 返回的十进制 token；`arbitrary_precision`
    /// 保证反序列化不经过 `f64`。
    Number(Number),
}

/// Binance USDⓈ-M `/fapi/v1/exchangeInfo` 的完整 wire 响应。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceExchangeInfo {
    /// Binance 使用的时区标识；通常为 `UTC`，SDK 不据此换算业务时间。
    pub timezone: String,
    /// Provider 生成响应时的 Unix 毫秒时间戳。
    pub server_time: u64,
    /// Provider 声明的请求速率与订单速率限制。
    pub rate_limits: Vec<BinanceRateLimit>,
    /// 交易所级过滤器；当前通常为空，未知字段仍原样保留。
    pub exchange_filters: Vec<BinanceExchangeFilter>,
    /// 可作为 USDⓈ-M 保证金资产的 provider 元数据。
    pub assets: Vec<BinanceExchangeAsset>,
    /// 当前响应范围内的全部合约及其交易规则。
    pub symbols: Vec<BinanceExchangeSymbol>,
    /// Binance 后续增加的顶层字段，保留给 evidence/兼容审计。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Binance 在 exchangeInfo body 中声明的一条限频规则。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceRateLimit {
    /// 限频类型，例如 `REQUEST_WEIGHT` 或 `ORDERS`；未知值不得在 SDK 拒绝。
    pub rate_limit_type: String,
    /// 计数窗口单位，例如 `SECOND` 或 `MINUTE`。
    pub interval: String,
    /// 一个窗口包含的 `interval` 数量。
    pub interval_num: u64,
    /// 该窗口允许的最大请求权重或订单数。
    pub limit: u64,
    /// Binance 后续增加的限频字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Binance 交易所级过滤器。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceExchangeFilter {
    /// Provider 过滤器类型；`None` 表示 Binance 尚未给该对象定义类型。
    pub filter_type: Option<String>,
    /// 过滤器的 provider 原始字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Binance USDⓈ-M 保证金资产的 wire 元数据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceExchangeAsset {
    /// Binance 资产代码，例如 `USDT`。
    pub asset: String,
    /// `true` 表示该资产允许参与 Multi-Assets margin；不代表用户账户已启用。
    pub margin_available: bool,
    /// Multi-Assets 模式自动兑换阈值。
    /// `None` 表示 provider 返回 `null` 或不为该资产配置阈值。
    pub auto_asset_exchange: Option<BinanceWireDecimal>,
    /// Binance 后续增加的资产字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Binance USDⓈ-M 单个合约及其 provider 交易规则。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceExchangeSymbol {
    /// Binance 下单与行情接口使用的 symbol，例如 `BTCUSDT`。
    pub symbol: String,
    /// 合约交易对代码；可能与 `symbol` 相同，但语义由 provider 决定。
    pub pair: String,
    /// 合约类型，例如 `PERPETUAL`；保留字符串以兼容新增 provider 状态。
    pub contract_type: String,
    /// 合约交割时间，Unix 毫秒；永续合约仍可能返回 provider 哨兵时间。
    pub delivery_date: u64,
    /// 合约上线时间，Unix 毫秒。
    pub onboard_date: u64,
    /// Provider 生命周期状态，例如 `TRADING`；SDK 不映射业务 readiness。
    pub status: String,
    /// Provider 维护保证金百分比文本；不应用于客户端风险计算。
    pub maint_margin_percent: BinanceWireDecimal,
    /// Provider 初始保证金百分比文本；不应用于客户端风险计算。
    pub required_margin_percent: BinanceWireDecimal,
    /// 标的资产代码，例如 `BTC`。
    pub base_asset: String,
    /// 报价资产代码，例如 `USDT`。
    pub quote_asset: String,
    /// 保证金结算资产代码，例如 `USDT`。
    pub margin_asset: String,
    /// Provider 展示用价格小数位；真实下单量化必须读取 `PRICE_FILTER.tickSize`。
    pub price_precision: u32,
    /// Provider 展示用数量小数位；真实下单量化必须读取 `LOT_SIZE.stepSize`。
    pub quantity_precision: u32,
    /// 标的资产精度位数。
    pub base_asset_precision: u32,
    /// 报价资产精度位数。
    pub quote_precision: u32,
    /// Provider 标的分类，例如 `COIN`。
    pub underlying_type: String,
    /// Provider 标的子分类列表；未知分类原样保留。
    pub underlying_sub_type: Vec<String>,
    /// Provider 结算计划编号。
    pub settle_plan: u64,
    /// 条件单触发保护比例的 provider 十进制表示。
    pub trigger_protect: BinanceWireDecimal,
    /// 合约适用的价格、数量、名义金额和订单数量过滤器。
    pub filters: Vec<BinanceSymbolFilter>,
    /// Provider 支持的订单类型。
    #[serde(alias = "OrderType")]
    pub order_types: Vec<String>,
    /// Provider 支持的有效期类型。
    pub time_in_force: Vec<String>,
    /// 强平手续费率的 provider 十进制表示。
    pub liquidation_fee: BinanceWireDecimal,
    /// 市价单成交价格边界的 provider 十进制表示。
    pub market_take_bound: BinanceWireDecimal,
    /// Binance 后续增加的 symbol 字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Binance symbol filter 的稳定已知字段与未知字段集合。
///
/// 不使用按 `filterType` 封闭分派的 enum，避免 Binance 新增过滤器时整批响应
/// 被 SDK 拒绝；`filter_type` 本身仍为必填，缺失会令全量响应 fail-closed。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinanceSymbolFilter {
    /// Provider 过滤器类型，例如 `PRICE_FILTER`、`LOT_SIZE` 或 `MIN_NOTIONAL`。
    pub filter_type: String,
    /// `PRICE_FILTER` 允许的最低价格。
    pub min_price: Option<BinanceWireDecimal>,
    /// `PRICE_FILTER` 允许的最高价格。
    pub max_price: Option<BinanceWireDecimal>,
    /// `PRICE_FILTER` 的价格步长。
    pub tick_size: Option<BinanceWireDecimal>,
    /// 数量过滤器允许的最小合约数量。
    pub min_qty: Option<BinanceWireDecimal>,
    /// 数量过滤器允许的最大合约数量。
    pub max_qty: Option<BinanceWireDecimal>,
    /// 数量过滤器的合约数量步长。
    pub step_size: Option<BinanceWireDecimal>,
    /// 订单数量过滤器的最大订单数。
    pub limit: Option<u64>,
    /// 最小名义金额过滤器的报价资产金额。
    pub notional: Option<BinanceWireDecimal>,
    /// 百分比价格过滤器允许的上界乘数。
    pub multiplier_up: Option<BinanceWireDecimal>,
    /// 百分比价格过滤器允许的下界乘数。
    pub multiplier_down: Option<BinanceWireDecimal>,
    /// 百分比价格乘数的小数位规则。
    pub multiplier_decimal: Option<BinanceWireDecimal>,
    /// Binance 后续增加的过滤器字段。
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
