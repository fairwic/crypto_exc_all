#[cfg(feature = "full-sdk")]
pub mod account;
#[cfg(feature = "full-sdk")]
pub mod adapters;
#[cfg(feature = "full-sdk")]
pub mod config;
pub mod error;
pub mod exchange;
#[cfg(feature = "full-sdk")]
pub mod fill;
pub mod instrument;
#[cfg(feature = "full-sdk")]
pub mod margin;
#[cfg(feature = "full-sdk")]
pub mod market;
#[cfg(feature = "full-sdk")]
pub mod order;
#[cfg(feature = "full-sdk")]
pub mod platform;
#[cfg(feature = "full-sdk")]
pub mod position;
#[cfg(any(feature = "binance", feature = "okx-public-market"))]
pub mod public_instrument;
#[cfg(feature = "okx-public-market")]
pub mod public_market;
#[cfg(feature = "full-sdk")]
pub mod sdk;
#[cfg(feature = "full-sdk")]
pub mod trade;

#[cfg(feature = "full-sdk")]
pub mod raw {
    #[cfg(feature = "binance")]
    pub use binance_rs as binance;

    #[cfg(feature = "bitget")]
    pub use bitget_rs as bitget;

    #[cfg(feature = "bybit")]
    pub use bybit_rs as bybit;

    #[cfg(feature = "gate")]
    pub use gate_rs as gate;

    #[cfg(feature = "hyperliquid")]
    pub use hyperliquid_rs as hyperliquid;

    #[cfg(feature = "okx")]
    pub use okx_rs as okx;
}

#[cfg(feature = "full-sdk")]
pub use account::{
    AccountBill, AccountBillQuery, AccountCapabilities, AccountFacade, Balance,
    EnsureOrderMarginModeRequest, EnsureOrderMarginModeResult, LeverageSetting,
    MarginModeApplyMethod, MaxOrderSize, MaxOrderSizeRequest, PositionMode, PositionModeSetting,
    PrepareOrderSettingsRequest, PrepareOrderSettingsResult, SetLeverageRequest,
    SetPositionModeRequest, SetSymbolMarginModeRequest, SymbolMarginModeSetting,
};
#[cfg(feature = "full-sdk")]
pub use config::{
    BinanceExchangeConfig, BitgetExchangeConfig, BybitExchangeConfig, GateExchangeConfig,
    HyperliquidExchangeConfig, OkxExchangeConfig, SdkConfig,
};
pub use error::{Error, Result};
pub use exchange::ExchangeId;
#[cfg(feature = "full-sdk")]
pub use fill::{Fill, FillFacade, FillListQuery};
pub use instrument::{Instrument, MarketType};
#[cfg(feature = "full-sdk")]
pub use margin::MarginMode;
#[cfg(feature = "full-sdk")]
pub use market::{
    Candle, CandleQuery, FundingRate, FundingRateQuery, LongShortRatio, MarkPrice, MarketFacade,
    MarketStatsQuery, OpenInterest, OrderBook, OrderBookLevel, OrderBookQuery, TakerBuySellVolume,
    Ticker,
};
#[cfg(feature = "full-sdk")]
pub use order::{Order, OrderFacade, OrderListQuery, OrderQuery};
#[cfg(feature = "full-sdk")]
pub use platform::{PlatformEvent, PlatformEventQuery, PlatformFacade};
#[cfg(feature = "full-sdk")]
pub use position::{Position, PositionFacade, PositionHistory, PositionHistoryQuery};
#[cfg(any(feature = "binance", feature = "okx-public-market"))]
pub use public_instrument::*;
#[cfg(feature = "okx-public-market")]
pub use public_market::{
    OKX_MAX_CANDLE_PAGE_SIZE, OkxCandleDataset, OkxPublicCandle, OkxPublicCandleQuery,
    OkxPublicMarketClient, OkxPublicMarketConfig,
};
#[cfg(feature = "full-sdk")]
pub use sdk::CryptoSdk;
#[cfg(feature = "full-sdk")]
pub use trade::{
    CancelOrderRequest, OrderAck, OrderSide, OrderType, PlaceOrderRequest, ProtectiveOrderQuery,
    ProtectiveOrderRequest, ProtectiveOrderWorkingType, TimeInForce, TradeCapabilities,
    TradeFacade,
};
