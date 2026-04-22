pub mod account;
pub mod adapters;
pub mod config;
pub mod error;
pub mod exchange;
pub mod fill;
pub mod instrument;
pub mod margin;
pub mod market;
pub mod order;
pub mod position;
pub mod sdk;
pub mod trade;

pub mod raw {
    #[cfg(feature = "binance")]
    pub use binance_rs as binance;

    #[cfg(feature = "bitget")]
    pub use bitget_rs as bitget;

    #[cfg(feature = "okx")]
    pub use okx_rs as okx;
}

pub use account::{
    AccountCapabilities, AccountFacade, Balance, EnsureOrderMarginModeRequest,
    EnsureOrderMarginModeResult, LeverageSetting, MarginModeApplyMethod, PositionMode,
    PositionModeSetting, PrepareOrderSettingsRequest, PrepareOrderSettingsResult,
    SetLeverageRequest, SetPositionModeRequest, SetSymbolMarginModeRequest,
    SymbolMarginModeSetting,
};
pub use config::{BinanceExchangeConfig, BitgetExchangeConfig, OkxExchangeConfig, SdkConfig};
pub use error::{Error, Result};
pub use exchange::ExchangeId;
pub use fill::{Fill, FillFacade, FillListQuery};
pub use instrument::{Instrument, MarketType};
pub use margin::MarginMode;
pub use market::{
    Candle, CandleQuery, FundingRate, FundingRateQuery, LongShortRatio, MarkPrice, MarketFacade,
    MarketStatsQuery, OpenInterest, OrderBook, OrderBookLevel, OrderBookQuery, TakerBuySellVolume,
    Ticker,
};
pub use order::{Order, OrderFacade, OrderListQuery, OrderQuery};
pub use position::{Position, PositionFacade};
pub use sdk::CryptoSdk;
pub use trade::{
    CancelOrderRequest, OrderAck, OrderSide, OrderType, PlaceOrderRequest, TimeInForce, TradeFacade,
};
