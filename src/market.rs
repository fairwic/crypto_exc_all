use crate::adapters::ExchangeClient;
use crate::error::Result;
use crate::exchange::ExchangeId;
use crate::instrument::Instrument;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ticker {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub last_price: String,
    pub bid_price: Option<String>,
    pub ask_price: Option<String>,
    pub volume_24h: Option<String>,
    pub timestamp: Option<u64>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderBookLevel {
    pub price: String,
    pub size: String,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrderBook {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub bids: Vec<OrderBookLevel>,
    pub asks: Vec<OrderBookLevel>,
    pub timestamp: Option<u64>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OrderBookQuery {
    pub instrument: Instrument,
    pub limit: Option<u32>,
}

impl OrderBookQuery {
    pub fn new(instrument: Instrument) -> Self {
        Self {
            instrument,
            limit: None,
        }
    }

    pub fn with_limit(mut self, value: u32) -> Self {
        self.limit = Some(value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Candle {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub open_time: Option<u64>,
    pub close_time: Option<u64>,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    pub quote_volume: Option<String>,
    pub closed: Option<bool>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandleQuery {
    pub instrument: Instrument,
    pub interval: String,
    pub limit: Option<u32>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
}

impl CandleQuery {
    pub fn new(instrument: Instrument, interval: impl Into<String>) -> Self {
        Self {
            instrument,
            interval: interval.into(),
            limit: None,
            after: None,
            before: None,
            start_time: None,
            end_time: None,
        }
    }

    pub fn with_limit(mut self, value: u32) -> Self {
        self.limit = Some(value);
        self
    }

    pub fn with_after(mut self, value: impl Into<String>) -> Self {
        self.after = Some(value.into());
        self
    }

    pub fn with_before(mut self, value: impl Into<String>) -> Self {
        self.before = Some(value.into());
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FundingRate {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub funding_rate: String,
    pub funding_time: Option<u64>,
    pub next_funding_rate: Option<String>,
    pub next_funding_time: Option<u64>,
    pub mark_price: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FundingRateQuery {
    pub instrument: Instrument,
    pub limit: Option<u32>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub after: Option<String>,
    pub before: Option<String>,
}

impl FundingRateQuery {
    pub fn new(instrument: Instrument) -> Self {
        Self {
            instrument,
            limit: None,
            start_time: None,
            end_time: None,
            after: None,
            before: None,
        }
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

    pub fn with_after(mut self, value: impl Into<String>) -> Self {
        self.after = Some(value.into());
        self
    }

    pub fn with_before(mut self, value: impl Into<String>) -> Self {
        self.before = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarkPrice {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub mark_price: String,
    pub index_price: Option<String>,
    pub funding_rate: Option<String>,
    pub next_funding_time: Option<u64>,
    pub timestamp: Option<u64>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenInterest {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub open_interest: String,
    pub open_interest_value: Option<String>,
    pub timestamp: Option<u64>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketStatsQuery {
    pub instrument: Instrument,
    pub period: String,
    pub limit: Option<u32>,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
}

impl MarketStatsQuery {
    pub fn new(instrument: Instrument, period: impl Into<String>) -> Self {
        Self {
            instrument,
            period: period.into(),
            limit: None,
            start_time: None,
            end_time: None,
        }
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LongShortRatio {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub period: String,
    pub ratio: String,
    pub long_ratio: Option<String>,
    pub short_ratio: Option<String>,
    pub timestamp: Option<u64>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TakerBuySellVolume {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub period: String,
    pub buy_volume: String,
    pub sell_volume: String,
    pub buy_sell_ratio: Option<String>,
    pub timestamp: Option<u64>,
    pub raw: Value,
}

pub struct MarketFacade<'a> {
    pub(crate) client: &'a ExchangeClient,
}

impl<'a> MarketFacade<'a> {
    pub(crate) fn new(client: &'a ExchangeClient) -> Self {
        Self { client }
    }

    pub async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        self.client.ticker(instrument).await
    }

    pub async fn orderbook(&self, query: OrderBookQuery) -> Result<OrderBook> {
        self.client.orderbook(query).await
    }

    pub async fn candles(&self, query: CandleQuery) -> Result<Vec<Candle>> {
        self.client.candles(query).await
    }

    pub async fn funding_rate(&self, instrument: &Instrument) -> Result<FundingRate> {
        self.client.funding_rate(instrument).await
    }

    pub async fn funding_rate_history(&self, query: FundingRateQuery) -> Result<Vec<FundingRate>> {
        self.client.funding_rate_history(query).await
    }

    pub async fn mark_price(&self, instrument: &Instrument) -> Result<MarkPrice> {
        self.client.mark_price(instrument).await
    }

    pub async fn open_interest(&self, instrument: &Instrument) -> Result<OpenInterest> {
        self.client.open_interest(instrument).await
    }

    pub async fn long_short_ratio(&self, query: MarketStatsQuery) -> Result<Vec<LongShortRatio>> {
        self.client.long_short_ratio(query).await
    }

    pub async fn taker_buy_sell_volume(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<TakerBuySellVolume>> {
        self.client.taker_buy_sell_volume(query).await
    }
}
