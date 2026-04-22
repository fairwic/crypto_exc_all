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

    pub async fn set_leverage(&self, request: SetLeverageRequest) -> Result<LeverageSetting> {
        self.client.set_leverage(request).await
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
}
