use super::super::*;

impl ExchangeClient {
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
}
