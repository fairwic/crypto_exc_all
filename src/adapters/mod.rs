use crate::account::{
    AccountCapabilities, Balance, EnsureOrderMarginModeRequest, EnsureOrderMarginModeResult,
    LeverageSetting, PositionModeSetting, PrepareOrderSettingsRequest, PrepareOrderSettingsResult,
    SetLeverageRequest, SetPositionModeRequest, SetSymbolMarginModeRequest,
    SymbolMarginModeSetting,
};
use crate::config::{BinanceExchangeConfig, BitgetExchangeConfig, OkxExchangeConfig};
use crate::error::Result;
use crate::exchange::ExchangeId;
use crate::fill::{Fill, FillListQuery};
use crate::instrument::Instrument;
use crate::market::{
    Candle, CandleQuery, FundingRate, FundingRateQuery, LongShortRatio, MarkPrice,
    MarketStatsQuery, OpenInterest, OrderBook, OrderBookQuery, TakerBuySellVolume, Ticker,
};
use crate::order::{Order, OrderListQuery, OrderQuery};
use crate::position::Position;
use crate::trade::{CancelOrderRequest, OrderAck, PlaceOrderRequest};

#[cfg(feature = "binance")]
mod binance;
#[cfg(feature = "bitget")]
mod bitget;
#[cfg(feature = "okx")]
mod okx;

#[cfg(feature = "binance")]
pub(crate) use binance::BinanceAdapter;
#[cfg(feature = "bitget")]
pub(crate) use bitget::BitgetAdapter;
#[cfg(feature = "okx")]
pub(crate) use okx::OkxAdapter;

pub(crate) enum ExchangeClient {
    #[cfg(feature = "okx")]
    Okx(Box<OkxAdapter>),
    #[cfg(feature = "binance")]
    Binance(Box<BinanceAdapter>),
    #[cfg(feature = "bitget")]
    Bitget(Box<BitgetAdapter>),
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

    pub(crate) fn exchange_id(&self) -> ExchangeId {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(_) => ExchangeId::Okx,
            #[cfg(feature = "binance")]
            Self::Binance(_) => ExchangeId::Binance,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => ExchangeId::Bitget,
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

    pub(crate) async fn positions(&self, instrument: Option<&Instrument>) -> Result<Vec<Position>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.positions(instrument).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.positions(instrument).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.positions(instrument).await,
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
        }
    }
}
