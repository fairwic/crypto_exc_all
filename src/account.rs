use crate::adapters::ExchangeClient;
use crate::error::Result;
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use crate::margin::MarginMode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Balance {
    pub exchange: ExchangeId,
    pub asset: String,
    pub total: String,
    pub available: String,
    pub frozen: Option<String>,
    pub raw: Value,
}

/// 带 provider 更新时间的余额读取结果；旧 `Balance` 合同保持不变。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourcedBalance {
    /// 既有 canonical 余额事实。
    pub balance: Balance,
    /// 交易所报告的余额更新时间（Unix 毫秒）。
    pub source_updated_at_ms: u64,
}

/// Provider signed 账户端点返回的同一报价币保证金摘要。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountMarginSummary {
    /// 提供摘要的交易所。
    pub exchange: ExchangeId,
    /// 所有金额字段共同使用的报价币种。
    pub quote_currency: String,
    /// 包含未实现盈亏的账户权益。
    pub account_equity: String,
    /// Provider 当前报告的可用保证金。
    pub available_margin: String,
    /// 账户总初始保证金；provider 当前模式不提供时为空。
    pub initial_margin: Option<String>,
    /// 持仓占用的初始保证金；provider 当前模式不提供时为空。
    pub position_initial_margin: Option<String>,
    /// 未完成订单占用的初始保证金；provider 当前模式不提供时为空。
    pub open_order_initial_margin: Option<String>,
    /// Provider 可证明的最近账户状态更新时间（Unix 毫秒）。
    pub source_updated_at_ms: u64,
    /// 固定协议映射版本，供 Account owner 生成审计 hash。
    pub source_revision: String,
}

/// 交易所官方账户身份与账户级模式；不包含 API credential 或业务准入判断。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountIdentity {
    pub exchange: ExchangeId,
    /// 交易所返回的账户身份字段；Gateway 负责生成非敏感 fingerprint 后立即丢弃原值。
    pub provider_account_id: String,
    /// 主账户 identity；普通主账户或交易所未提供时为空。
    pub parent_account_id: Option<String>,
    /// 交易所账户级资金模式，不替代订单/持仓逐项 margin mode。
    pub margin_mode: String,
    /// 账户持仓模式。
    pub position_mode: String,
    /// 当前 facade 覆盖产品的结算币种。
    pub settlement_asset: String,
}

/// 交易所 signed account-config 返回的下单权限事实；不等同于 Core 业务准入。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountOrderPermission {
    pub exchange: ExchangeId,
    /// 当前 API credential/account configuration 是否允许创建订单。
    pub can_create_orders: bool,
    /// 固定映射版本，供上层生成不含原始 provider 配置的审计哈希。
    pub source_revision: String,
}

/// 账户身份，以及同一次 signed account-config 响应产生的账户模式与下单权限。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountOrderPermissionWithIdentity {
    pub identity: AccountIdentity,
    pub order_permission: AccountOrderPermission,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountBill {
    pub exchange: ExchangeId,
    pub instrument: Option<Instrument>,
    pub exchange_symbol: Option<String>,
    pub bill_id: Option<String>,
    pub asset: Option<String>,
    pub balance_change: Option<String>,
    pub balance_after: Option<String>,
    pub fee: Option<String>,
    pub pnl: Option<String>,
    pub bill_type: Option<String>,
    pub bill_sub_type: Option<String>,
    pub order_id: Option<String>,
    pub trade_id: Option<String>,
    pub timestamp: Option<u64>,
    pub raw: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountBillQuery {
    pub instrument: Option<Instrument>,
    pub asset: Option<String>,
    pub inst_type: Option<String>,
    pub bill_type: Option<String>,
    pub limit: Option<u32>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub archive: bool,
}

impl AccountBillQuery {
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

    pub fn with_asset(mut self, value: impl Into<String>) -> Self {
        self.asset = Some(value.into());
        self
    }

    pub fn with_inst_type(mut self, value: impl Into<String>) -> Self {
        self.inst_type = Some(value.into());
        self
    }

    pub fn with_bill_type(mut self, value: impl Into<String>) -> Self {
        self.bill_type = Some(value.into());
        self
    }

    pub fn with_limit(mut self, value: u32) -> Self {
        self.limit = Some(value);
        self
    }

    pub fn with_start_time(mut self, value: u64) -> Self {
        self.start_time = Some(value);
        self
    }

    pub fn with_end_time(mut self, value: u64) -> Self {
        self.end_time = Some(value);
        self
    }

    pub fn with_archive(mut self, value: bool) -> Self {
        self.archive = value;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetLeverageRequest {
    pub instrument: Instrument,
    pub leverage: String,
    pub margin_mode: Option<MarginMode>,
    pub margin_coin: Option<String>,
    pub position_side: Option<String>,
}

impl SetLeverageRequest {
    pub fn new(instrument: Instrument, leverage: impl Into<String>) -> Self {
        Self {
            instrument,
            leverage: leverage.into(),
            margin_mode: None,
            margin_coin: None,
            position_side: None,
        }
    }

    pub fn with_margin_mode(mut self, value: impl Into<MarginMode>) -> Self {
        self.margin_mode = Some(value.into());
        self
    }

    pub fn with_margin_coin(mut self, value: impl Into<String>) -> Self {
        self.margin_coin = Some(value.into());
        self
    }

    pub fn with_position_side(mut self, value: impl Into<String>) -> Self {
        self.position_side = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeverageSetting {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub leverage: String,
    pub margin_mode: Option<String>,
    pub margin_coin: Option<String>,
    pub position_side: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeverageInfoQuery {
    pub instrument: Instrument,
    pub margin_mode: MarginMode,
}

impl LeverageInfoQuery {
    pub fn new(instrument: Instrument, margin_mode: impl Into<MarginMode>) -> Self {
        Self {
            instrument,
            margin_mode: margin_mode.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MaxOrderSizeRequest {
    /// 需要查询最大可下单数量的交易产品。
    pub instrument: Instrument,
    /// 交易所下单模式；OKX 使用该值映射到 tdMode。
    pub margin_mode: MarginMode,
    /// 保证金币种；逐仓 U 本位合约通常为 USDT。
    pub margin_coin: Option<String>,
    /// 参考委托价；市价单用当前参考价帮助交易所计算保证金占用。
    pub price: Option<String>,
    /// 已设置或即将使用的杠杆倍数。
    pub leverage: Option<String>,
}

impl MaxOrderSizeRequest {
    /// 创建最大可下单数量查询请求；调用方应先完成对应的账户杠杆/保证金设置。
    pub fn new(instrument: Instrument, margin_mode: MarginMode) -> Self {
        Self {
            instrument,
            margin_mode,
            margin_coin: None,
            price: None,
            leverage: None,
        }
    }

    /// 设置保证金币种，用于逐仓或交易所要求显式传币种的账户查询。
    pub fn with_margin_coin(mut self, value: impl Into<String>) -> Self {
        self.margin_coin = Some(value.into());
        self
    }

    /// 设置参考价格，便于交易所按当前委托上下文计算最大可开数量。
    pub fn with_price(mut self, value: impl Into<String>) -> Self {
        self.price = Some(value.into());
        self
    }

    /// 设置杠杆倍数，必须与调用方已应用到账户的策略杠杆保持一致。
    pub fn with_leverage(mut self, value: impl Into<String>) -> Self {
        self.leverage = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaxOrderSize {
    /// 交易所名称。
    pub exchange: ExchangeId,
    /// 统一产品模型。
    pub instrument: Instrument,
    /// 交易所原始产品 ID。
    pub exchange_symbol: String,
    /// 查询使用的保证金模式。
    pub margin_mode: MarginMode,
    /// 交易所返回或请求传入的保证金币种。
    pub margin_coin: Option<String>,
    /// 最大可买数量，单位保持交易所下单单位。
    pub max_buy: String,
    /// 最大可卖数量，单位保持交易所下单单位。
    pub max_sell: String,
    /// 原始响应，供审计和排障使用。
    pub raw: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountCapabilities {
    pub set_leverage: bool,
    pub set_position_mode: bool,
    pub set_symbol_margin_mode: bool,
    pub order_level_margin_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetSymbolMarginModeRequest {
    pub instrument: Instrument,
    pub mode: MarginMode,
    pub product_type: Option<String>,
    pub margin_coin: Option<String>,
}

impl SetSymbolMarginModeRequest {
    pub fn new(instrument: Instrument, mode: MarginMode) -> Self {
        Self {
            instrument,
            mode,
            product_type: None,
            margin_coin: None,
        }
    }

    pub fn with_product_type(mut self, value: impl Into<String>) -> Self {
        self.product_type = Some(value.into());
        self
    }

    pub fn with_margin_coin(mut self, value: impl Into<String>) -> Self {
        self.margin_coin = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SymbolMarginModeSetting {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub mode: MarginMode,
    pub raw_mode: Option<String>,
    pub product_type: Option<String>,
    pub margin_coin: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarginModeApplyMethod {
    SymbolConfiguration,
    OrderLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnsureOrderMarginModeRequest {
    pub instrument: Instrument,
    pub mode: MarginMode,
    pub product_type: Option<String>,
    pub margin_coin: Option<String>,
}

impl EnsureOrderMarginModeRequest {
    pub fn new(instrument: Instrument, mode: MarginMode) -> Self {
        Self {
            instrument,
            mode,
            product_type: None,
            margin_coin: None,
        }
    }

    pub fn with_product_type(mut self, value: impl Into<String>) -> Self {
        self.product_type = Some(value.into());
        self
    }

    pub fn with_margin_coin(mut self, value: impl Into<String>) -> Self {
        self.margin_coin = Some(value.into());
        self
    }

    pub(crate) fn into_set_symbol_request(self) -> SetSymbolMarginModeRequest {
        SetSymbolMarginModeRequest {
            instrument: self.instrument,
            mode: self.mode,
            product_type: self.product_type,
            margin_coin: self.margin_coin,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnsureOrderMarginModeResult {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub mode: MarginMode,
    pub apply_method: MarginModeApplyMethod,
    pub raw_mode: Option<String>,
    pub product_type: Option<String>,
    pub margin_coin: Option<String>,
    pub raw: Value,
}

impl EnsureOrderMarginModeResult {
    pub(crate) fn from_symbol_setting(setting: SymbolMarginModeSetting) -> Self {
        Self {
            exchange: setting.exchange,
            instrument: setting.instrument,
            exchange_symbol: setting.exchange_symbol,
            mode: setting.mode,
            apply_method: MarginModeApplyMethod::SymbolConfiguration,
            raw_mode: setting.raw_mode,
            product_type: setting.product_type,
            margin_coin: setting.margin_coin,
            raw: setting.raw,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrepareOrderSettingsRequest {
    pub instrument: Instrument,
    pub margin_mode: Option<MarginMode>,
    pub leverage: Option<String>,
    pub position_mode: Option<PositionMode>,
    pub product_type: Option<String>,
    pub margin_coin: Option<String>,
    pub position_side: Option<String>,
}

impl PrepareOrderSettingsRequest {
    pub fn new(instrument: Instrument) -> Self {
        Self {
            instrument,
            margin_mode: None,
            leverage: None,
            position_mode: None,
            product_type: None,
            margin_coin: None,
            position_side: None,
        }
    }

    pub fn with_margin_mode(mut self, value: impl Into<MarginMode>) -> Self {
        self.margin_mode = Some(value.into());
        self
    }

    pub fn with_leverage(mut self, value: impl Into<String>) -> Self {
        self.leverage = Some(value.into());
        self
    }

    pub fn with_position_mode(mut self, value: PositionMode) -> Self {
        self.position_mode = Some(value);
        self
    }

    pub fn with_product_type(mut self, value: impl Into<String>) -> Self {
        self.product_type = Some(value.into());
        self
    }

    pub fn with_margin_coin(mut self, value: impl Into<String>) -> Self {
        self.margin_coin = Some(value.into());
        self
    }

    pub fn with_position_side(mut self, value: impl Into<String>) -> Self {
        self.position_side = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PrepareOrderSettingsResult {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub position_mode: Option<PositionModeSetting>,
    pub margin_mode: Option<EnsureOrderMarginModeResult>,
    pub leverage: Option<LeverageSetting>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PositionMode {
    OneWay,
    Hedge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetPositionModeRequest {
    pub mode: PositionMode,
    pub product_type: Option<String>,
}

impl SetPositionModeRequest {
    pub fn new(mode: PositionMode) -> Self {
        Self {
            mode,
            product_type: None,
        }
    }

    pub fn with_product_type(mut self, value: impl Into<String>) -> Self {
        self.product_type = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PositionModeSetting {
    pub exchange: ExchangeId,
    pub mode: PositionMode,
    pub raw_mode: Option<String>,
    pub product_type: Option<String>,
    pub raw: Value,
}

pub struct AccountFacade<'a> {
    pub(crate) client: &'a ExchangeClient,
}

impl<'a> AccountFacade<'a> {
    pub(crate) fn new(client: &'a ExchangeClient) -> Self {
        Self { client }
    }

    pub async fn balances(&self) -> Result<Vec<Balance>> {
        self.client.balances().await
    }

    /// 读取带 provider 更新时间的 OKX/Binance 余额，供 snapshot/stream 因果合并。
    pub async fn sourced_balances(&self) -> Result<Vec<SourcedBalance>> {
        self.client.sourced_balances().await
    }

    /// 读取同一报价币的 signed 保证金摘要；无法等价表达时返回错误。
    pub async fn margin_summary(&self, quote_currency: &str) -> Result<AccountMarginSummary> {
        self.client.margin_summary(quote_currency).await
    }

    /// 读取官方 signed account identity；本方法只做协议映射，不判断用户授权或 Core admission。
    pub async fn identity(&self) -> Result<AccountIdentity> {
        self.client.account_identity().await
    }

    /// 读取 provider 的 signed 下单权限；用户授权、账户状态与风险判断仍由 Core 负责。
    pub async fn order_permission(&self) -> Result<AccountOrderPermission> {
        self.client.account_order_permission().await
    }

    /// 使用同一次 signed account-config 响应读取账户模式与下单权限。
    pub async fn order_permission_with_identity(
        &self,
    ) -> Result<AccountOrderPermissionWithIdentity> {
        self.client.account_order_permission_with_identity().await
    }

    pub async fn bills(&self, query: AccountBillQuery) -> Result<Vec<AccountBill>> {
        self.client.account_bills(query).await
    }

    pub async fn set_leverage(&self, request: SetLeverageRequest) -> Result<LeverageSetting> {
        self.client.set_leverage(request).await
    }

    /// 读取 provider 当前杠杆配置，不修改账户设置。
    pub async fn leverage_info(&self, query: LeverageInfoQuery) -> Result<Vec<LeverageSetting>> {
        self.client.leverage_info(query).await
    }

    pub fn capabilities(&self) -> AccountCapabilities {
        self.client.account_capabilities()
    }

    pub async fn set_position_mode(
        &self,
        request: SetPositionModeRequest,
    ) -> Result<PositionModeSetting> {
        self.client.set_position_mode(request).await
    }

    pub async fn set_symbol_margin_mode(
        &self,
        request: SetSymbolMarginModeRequest,
    ) -> Result<SymbolMarginModeSetting> {
        self.client.set_symbol_margin_mode(request).await
    }

    pub async fn ensure_order_margin_mode(
        &self,
        request: EnsureOrderMarginModeRequest,
    ) -> Result<EnsureOrderMarginModeResult> {
        self.client.ensure_order_margin_mode(request).await
    }

    pub async fn prepare_order_settings(
        &self,
        request: PrepareOrderSettingsRequest,
    ) -> Result<PrepareOrderSettingsResult> {
        self.client.prepare_order_settings(request).await
    }

    /// 查询账户当前最大可下单数量；调用方负责保证账户杠杆/保证金设置已完成。
    pub async fn max_order_size(&self, request: MaxOrderSizeRequest) -> Result<MaxOrderSize> {
        self.client.max_order_size(request).await
    }
}
