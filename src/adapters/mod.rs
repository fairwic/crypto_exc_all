use crate::account::{
    AccountBill, AccountBillQuery, AccountCapabilities, AccountIdentity, AccountMarginSummary,
    AccountOrderPermission, Balance, EnsureOrderMarginModeRequest, EnsureOrderMarginModeResult,
    LeverageSetting, MaxOrderSize, MaxOrderSizeRequest, PositionModeSetting,
    PrepareOrderSettingsRequest, PrepareOrderSettingsResult, SetLeverageRequest,
    SetPositionModeRequest, SetSymbolMarginModeRequest, SourcedBalance, SymbolMarginModeSetting,
};
use crate::config::{
    BinanceExchangeConfig, BitgetExchangeConfig, BybitExchangeConfig, GateExchangeConfig,
    HyperliquidExchangeConfig, OkxExchangeConfig,
};
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::fill::{Fill, FillListQuery};
use crate::instrument::Instrument;
use crate::market::{
    Candle, CandleQuery, FundingRate, FundingRateQuery, LongShortRatio, MarkPrice,
    MarketStatsQuery, OpenInterest, OrderBook, OrderBookQuery, TakerBuySellVolume, Ticker,
};
use crate::order::{Order, OrderListQuery, OrderQuery};
use crate::platform::{PlatformEvent, PlatformEventQuery};
use crate::position::{Position, PositionHistory, PositionHistoryQuery, SourcedPosition};
use crate::private_account_stream::PrivateAccountStreamSession;
use crate::trade::{
    CancelOrderRequest, OrderAck, PlaceOrderRequest, ProtectiveOrderQuery, ProtectiveOrderRequest,
    TradeCapabilities,
};

#[cfg(feature = "binance")]
mod binance;
#[cfg(feature = "bitget")]
mod bitget;
#[cfg(feature = "bybit")]
mod bybit;
#[cfg(feature = "gate")]
mod gate;
#[cfg(feature = "hyperliquid")]
mod hyperliquid;
#[cfg(feature = "hyperliquid")]
mod hyperliquid_bills;
#[cfg(feature = "hyperliquid")]
mod hyperliquid_market;
#[cfg(feature = "hyperliquid")]
mod hyperliquid_orders;
#[cfg(feature = "hyperliquid")]
mod hyperliquid_spot;
#[cfg(feature = "okx")]
mod okx;
mod value;

#[cfg(feature = "binance")]
pub(crate) use binance::BinanceAdapter;
#[cfg(feature = "bitget")]
pub(crate) use bitget::BitgetAdapter;
#[cfg(feature = "bybit")]
pub(crate) use bybit::BybitAdapter;
#[cfg(feature = "gate")]
pub(crate) use gate::GateAdapter;
#[cfg(feature = "hyperliquid")]
pub(crate) use hyperliquid::HyperliquidAdapter;
#[cfg(feature = "okx")]
pub(crate) use okx::OkxAdapter;

pub(crate) enum ExchangeClient {
    #[cfg(feature = "okx")]
    Okx(Box<OkxAdapter>),
    #[cfg(feature = "binance")]
    Binance(Box<BinanceAdapter>),
    #[cfg(feature = "bitget")]
    Bitget(Box<BitgetAdapter>),
    #[cfg(feature = "bybit")]
    Bybit(Box<BybitAdapter>),
    #[cfg(feature = "gate")]
    Gate(Box<GateAdapter>),
    #[cfg(feature = "hyperliquid")]
    Hyperliquid(Box<HyperliquidAdapter>),
}

impl ExchangeClient {
    #[cfg(feature = "okx")]
    pub(crate) fn okx(config: OkxExchangeConfig) -> Result<Self> {
        Ok(Self::Okx(Box::new(OkxAdapter::new(config)?)))
    }

    #[cfg(feature = "binance")]
    pub(crate) fn binance(config: BinanceExchangeConfig) -> Result<Self> {
        Ok(Self::Binance(Box::new(BinanceAdapter::new(config)?)))
    }

    #[cfg(feature = "bitget")]
    pub(crate) fn bitget(config: BitgetExchangeConfig) -> Result<Self> {
        Ok(Self::Bitget(Box::new(BitgetAdapter::new(config)?)))
    }

    #[cfg(feature = "bybit")]
    pub(crate) fn bybit(config: BybitExchangeConfig) -> Result<Self> {
        Ok(Self::Bybit(Box::new(BybitAdapter::new(config)?)))
    }

    #[cfg(feature = "gate")]
    pub(crate) fn gate(config: GateExchangeConfig) -> Result<Self> {
        Ok(Self::Gate(Box::new(GateAdapter::new(config)?)))
    }

    #[cfg(feature = "hyperliquid")]
    pub(crate) fn hyperliquid(config: HyperliquidExchangeConfig) -> Result<Self> {
        Ok(Self::Hyperliquid(Box::new(HyperliquidAdapter::new(
            config,
        )?)))
    }

    pub(crate) fn exchange_id(&self) -> ExchangeId {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(_) => ExchangeId::Okx,
            #[cfg(feature = "binance")]
            Self::Binance(_) => ExchangeId::Binance,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => ExchangeId::Bitget,
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => ExchangeId::Bybit,
            #[cfg(feature = "gate")]
            Self::Gate(_) => ExchangeId::Gate,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => ExchangeId::Hyperliquid,
        }
    }

    pub(crate) async fn open_private_account_stream(&self) -> Result<PrivateAccountStreamSession> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.open_private_account_stream().await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.open_private_account_stream().await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "private_account_stream",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "private_account_stream",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "private_account_stream",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "private_account_stream",
            }),
        }
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.ticker(instrument).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.ticker(instrument).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.ticker(instrument).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.ticker(instrument).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.ticker(instrument).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.ticker(instrument).await,
        }
    }

    pub(crate) async fn tickers(&self, instrument_type: &str) -> Result<Vec<Ticker>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.tickers(instrument_type).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.tickers(instrument_type).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.tickers(instrument_type).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.tickers(instrument_type).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.tickers(instrument_type).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.tickers(instrument_type).await,
        }
    }

    pub(crate) async fn orderbook(&self, query: OrderBookQuery) -> Result<OrderBook> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.orderbook(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.orderbook(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.orderbook(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.orderbook(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.orderbook(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.orderbook(query).await,
        }
    }

    pub(crate) async fn platform_system_status(
        &self,
        query: PlatformEventQuery,
    ) -> Result<Vec<PlatformEvent>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.platform_system_status(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.platform_system_status(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.platform_system_status(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.platform_system_status(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "platform system status",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "platform system status",
            }),
        }
    }

    pub(crate) async fn platform_announcements(
        &self,
        query: PlatformEventQuery,
    ) -> Result<Vec<PlatformEvent>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.platform_announcements(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.platform_announcements(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.platform_announcements(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.platform_announcements(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "platform announcements",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "platform announcements",
            }),
        }
    }

    pub(crate) async fn candles(&self, query: CandleQuery) -> Result<Vec<Candle>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.candles(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.candles(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.candles(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.candles(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.candles(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.candles(query).await,
        }
    }

    pub(crate) async fn funding_rate(&self, instrument: &Instrument) -> Result<FundingRate> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.funding_rate(instrument).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.funding_rate(instrument).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.funding_rate(instrument).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.funding_rate(instrument).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.funding_rate(instrument).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.funding_rate(instrument).await,
        }
    }

    pub(crate) async fn funding_rate_history(
        &self,
        query: FundingRateQuery,
    ) -> Result<Vec<FundingRate>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.funding_rate_history(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.funding_rate_history(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.funding_rate_history(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.funding_rate_history(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.funding_rate_history(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.funding_rate_history(query).await,
        }
    }

    pub(crate) async fn mark_price(&self, instrument: &Instrument) -> Result<MarkPrice> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.mark_price(instrument).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.mark_price(instrument).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.mark_price(instrument).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.mark_price(instrument).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.mark_price(instrument).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.mark_price(instrument).await,
        }
    }

    pub(crate) async fn open_interest(&self, instrument: &Instrument) -> Result<OpenInterest> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.open_interest(instrument).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.open_interest(instrument).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.open_interest(instrument).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.open_interest(instrument).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.open_interest(instrument).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.open_interest(instrument).await,
        }
    }

    pub(crate) async fn open_interest_history(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<OpenInterest>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Okx,
                capability: "open interest history",
            }),
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.open_interest_history(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "open interest history",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.open_interest_history(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "open interest history",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "open interest history",
            }),
        }
    }

    pub(crate) async fn long_short_ratio(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<LongShortRatio>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.long_short_ratio(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.long_short_ratio(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.long_short_ratio(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.long_short_ratio(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.long_short_ratio(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "long-short ratio",
            }),
        }
    }

    pub(crate) async fn top_trader_position_ratio(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<LongShortRatio>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.top_trader_position_ratio(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.top_trader_position_ratio(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "top trader position ratio",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "top trader position ratio",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "top trader position ratio",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "top trader position ratio",
            }),
        }
    }

    pub(crate) async fn taker_buy_sell_volume(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<TakerBuySellVolume>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.taker_buy_sell_volume(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.taker_buy_sell_volume(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.taker_buy_sell_volume(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.taker_buy_sell_volume(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.taker_buy_sell_volume(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "taker buy-sell volume",
            }),
        }
    }

    pub(crate) async fn balances(&self) -> Result<Vec<Balance>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.balances().await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.balances().await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.balances().await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.balances().await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.balances().await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.balances().await,
        }
    }

    /// 只有首版 Account owner 支持的 OKX/Binance 暴露 source time 合同。
    pub(crate) async fn sourced_balances(&self) -> Result<Vec<SourcedBalance>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.sourced_balances().await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.sourced_balances().await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "sourced account balances",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "sourced account balances",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "sourced account balances",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "sourced account balances",
            }),
        }
    }

    /// 只有 OKX/Binance 能提供当前 Account owner 所需的 signed 保证金摘要。
    pub(crate) async fn margin_summary(
        &self,
        quote_currency: &str,
    ) -> Result<AccountMarginSummary> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.margin_summary(quote_currency).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.margin_summary(quote_currency).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "account margin summary",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "account margin summary",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "account margin summary",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "account margin summary",
            }),
        }
    }

    pub(crate) async fn account_identity(&self) -> Result<AccountIdentity> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.account_identity().await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.account_identity().await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "account identity",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "account identity",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "account identity",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "account identity",
            }),
        }
    }

    pub(crate) async fn account_order_permission(&self) -> Result<AccountOrderPermission> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.account_order_permission().await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.account_order_permission().await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "account order permission",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "account order permission",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "account order permission",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "account order permission",
            }),
        }
    }

    pub(crate) async fn account_bills(&self, query: AccountBillQuery) -> Result<Vec<AccountBill>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.account_bills(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.account_bills(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.account_bills(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.account_bills(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.account_bills(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.account_bills(query).await,
        }
    }

    pub(crate) async fn set_leverage(
        &self,
        request: SetLeverageRequest,
    ) -> Result<LeverageSetting> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.set_leverage(request).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.set_leverage(request).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.set_leverage(request).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.set_leverage(request).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.set_leverage(request).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "set leverage",
            }),
        }
    }

    pub(crate) fn account_capabilities(&self) -> AccountCapabilities {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.account_capabilities(),
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.account_capabilities(),
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.account_capabilities(),
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.account_capabilities(),
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.account_capabilities(),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.account_capabilities(),
        }
    }

    pub(crate) async fn set_position_mode(
        &self,
        request: SetPositionModeRequest,
    ) -> Result<PositionModeSetting> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.set_position_mode(request).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.set_position_mode(request).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.set_position_mode(request).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.set_position_mode(request).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.set_position_mode(request).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "set position mode",
            }),
        }
    }

    pub(crate) async fn set_symbol_margin_mode(
        &self,
        request: SetSymbolMarginModeRequest,
    ) -> Result<SymbolMarginModeSetting> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.set_symbol_margin_mode(request).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.set_symbol_margin_mode(request).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.set_symbol_margin_mode(request).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.set_symbol_margin_mode(request).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.set_symbol_margin_mode(request).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "set symbol margin mode",
            }),
        }
    }

    pub(crate) async fn ensure_order_margin_mode(
        &self,
        request: EnsureOrderMarginModeRequest,
    ) -> Result<EnsureOrderMarginModeResult> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.ensure_order_margin_mode(request).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.ensure_order_margin_mode(request).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.ensure_order_margin_mode(request).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.ensure_order_margin_mode(request).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.ensure_order_margin_mode(request).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "ensure order margin mode",
            }),
        }
    }

    pub(crate) async fn prepare_order_settings(
        &self,
        request: PrepareOrderSettingsRequest,
    ) -> Result<PrepareOrderSettingsResult> {
        let exchange = self.exchange_id();
        let instrument = request.instrument.clone();
        let exchange_symbol = instrument.symbol_for(exchange);

        let position_mode = if let Some(mode) = request.position_mode {
            Some(
                self.set_position_mode(SetPositionModeRequest {
                    mode,
                    product_type: request.product_type.clone(),
                })
                .await?,
            )
        } else {
            None
        };

        let margin_mode = if let Some(mode) = request.margin_mode.clone() {
            Some(
                self.ensure_order_margin_mode(EnsureOrderMarginModeRequest {
                    instrument: instrument.clone(),
                    mode,
                    product_type: request.product_type.clone(),
                    margin_coin: request.margin_coin.clone(),
                })
                .await?,
            )
        } else {
            None
        };

        let leverage = if let Some(leverage) = request.leverage.clone() {
            Some(
                self.set_leverage(SetLeverageRequest {
                    instrument: instrument.clone(),
                    leverage,
                    margin_mode: request.margin_mode,
                    margin_coin: request.margin_coin,
                    position_side: request.position_side,
                })
                .await?,
            )
        } else {
            None
        };

        Ok(PrepareOrderSettingsResult {
            exchange,
            instrument,
            exchange_symbol,
            position_mode,
            margin_mode,
            leverage,
        })
    }

    /// 读取账户最大可下单数量；当前仅 OKX adapter 暴露等价 signed read-only 能力。
    pub(crate) async fn max_order_size(
        &self,
        request: MaxOrderSizeRequest,
    ) -> Result<MaxOrderSize> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.max_order_size(request).await,
            #[cfg(feature = "binance")]
            Self::Binance(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Binance,
                capability: "account max order size",
            }),
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "account max order size",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "account max order size",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "account max order size",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "account max order size",
            }),
        }
    }

    pub(crate) async fn positions(&self, instrument: Option<&Instrument>) -> Result<Vec<Position>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.positions(instrument).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.positions(instrument).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.positions(instrument).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.positions(instrument).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.positions(instrument).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.positions(instrument).await,
        }
    }

    /// 只有首版 Account owner 支持的 OKX/Binance 暴露 source time 合同。
    pub(crate) async fn sourced_positions(
        &self,
        instrument: Option<&Instrument>,
    ) -> Result<Vec<SourcedPosition>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.sourced_positions(instrument).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.sourced_positions(instrument).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "sourced account positions",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "sourced account positions",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "sourced account positions",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "sourced account positions",
            }),
        }
    }

    pub(crate) async fn position_history(
        &self,
        query: PositionHistoryQuery,
    ) -> Result<Vec<PositionHistory>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.position_history(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Binance,
                capability: "position history",
            }),
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "position history",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "position history",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "position history",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "position history",
            }),
        }
    }

    pub(crate) async fn place_order(&self, request: PlaceOrderRequest) -> Result<OrderAck> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.place_order(request).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.place_order(request).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.place_order(request).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.place_order(request).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.place_order(request).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "place order",
            }),
        }
    }

    pub(crate) fn trade_capabilities(&self) -> TradeCapabilities {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(_) => TradeCapabilities {
                attached_stop_loss_on_place_order: true,
                protective_order: false,
            },
            #[cfg(feature = "binance")]
            Self::Binance(_) => TradeCapabilities {
                attached_stop_loss_on_place_order: false,
                protective_order: true,
            },
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => TradeCapabilities {
                attached_stop_loss_on_place_order: true,
                protective_order: false,
            },
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => TradeCapabilities {
                attached_stop_loss_on_place_order: false,
                protective_order: false,
            },
            #[cfg(feature = "gate")]
            Self::Gate(_) => TradeCapabilities {
                attached_stop_loss_on_place_order: false,
                protective_order: false,
            },
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => TradeCapabilities {
                attached_stop_loss_on_place_order: false,
                protective_order: false,
            },
        }
    }

    pub(crate) async fn place_protective_order(
        &self,
        request: ProtectiveOrderRequest,
    ) -> Result<OrderAck> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Okx,
                capability: "protective order",
            }),
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.place_protective_order(request).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "protective order",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "protective order",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "protective order",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "protective order",
            }),
        }
    }

    pub(crate) async fn cancel_order(&self, request: CancelOrderRequest) -> Result<OrderAck> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.cancel_order(request).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.cancel_order(request).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.cancel_order(request).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.cancel_order(request).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.cancel_order(request).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "cancel order",
            }),
        }
    }

    pub(crate) async fn cancel_protective_order(
        &self,
        request: CancelOrderRequest,
    ) -> Result<OrderAck> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Okx,
                capability: "protective order cancellation",
            }),
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.cancel_protective_order(request).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "protective order cancellation",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "protective order cancellation",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "protective order cancellation",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "protective order cancellation",
            }),
        }
    }

    pub(crate) async fn order(&self, query: OrderQuery) -> Result<Order> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.order(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.order(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.order(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.order(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.order(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.order(query).await,
        }
    }

    pub(crate) async fn protective_order(&self, query: ProtectiveOrderQuery) -> Result<Order> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.order(query.into_order_query()).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.protective_order(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.order(query.into_order_query()).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.order(query.into_order_query()).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.order(query.into_order_query()).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "protective order detail",
            }),
        }
    }

    pub(crate) async fn open_orders(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.open_orders(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.open_orders(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.open_orders(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.open_orders(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.open_orders(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.open_orders(query).await,
        }
    }

    pub(crate) async fn order_history(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.order_history(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.order_history(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.order_history(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.order_history(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.order_history(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.order_history(query).await,
        }
    }

    pub(crate) async fn fills(&self, query: FillListQuery) -> Result<Vec<Fill>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.fills(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.fills(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.fills(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.fills(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.fills(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.fills(query).await,
        }
    }
}
