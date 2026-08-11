use super::*;

impl OkxAdapter {
    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        let exchange = ExchangeId::Okx;
        let symbol = instrument.symbol_for(exchange);
        let mut tickers = self
            .market
            .get_ticker(&symbol)
            .await
            .map_err(Error::from_okx)?;
        let ticker = tickers.drain(..).next().ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("OKX ticker response is empty for {symbol}"),
        })?;
        okx_ticker_from_dto(exchange, Some(instrument.clone()), Some(symbol), ticker)
    }

    pub(crate) async fn tickers(&self, instrument_type: &str) -> Result<Vec<Ticker>> {
        let exchange = ExchangeId::Okx;
        let tickers = self
            .market
            .get_tickers(instrument_type)
            .await
            .map_err(Error::from_okx)?;

        tickers
            .into_iter()
            .map(|ticker| okx_ticker_from_dto(exchange, None, None, ticker))
            .collect()
    }

    pub(crate) async fn orderbook(&self, query: OrderBookQuery) -> Result<OrderBook> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let depth = self
            .market
            .get_books(&symbol, query.limit)
            .await
            .map_err(Error::from_okx)?;
        let raw = serde_json::to_value(&depth)?;

        Ok(OrderBook {
            exchange,
            instrument,
            exchange_symbol: if depth.inst_id.is_empty() {
                symbol
            } else {
                depth.inst_id
            },
            bids: okx_book_levels(depth.bids),
            asks: okx_book_levels(depth.asks),
            timestamp: parse_u64_string(&depth.ts),
            raw,
        })
    }

    pub(crate) async fn candles(&self, query: CandleQuery) -> Result<Vec<Candle>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let interval = okx_candle_interval(&query.interval);
        let limit = query.limit.map(|value| value.to_string());
        let candles = self
            .market
            .get_candles(
                &symbol,
                &interval,
                query.after.as_deref(),
                query.before.as_deref(),
                limit.as_deref(),
            )
            .await
            .map_err(Error::from_okx)?;

        candles
            .into_iter()
            .map(|candle| okx_candle_from_dto(exchange, &instrument, &symbol, candle))
            .collect()
    }

    pub(crate) async fn funding_rate(&self, instrument: &Instrument) -> Result<FundingRate> {
        let exchange = ExchangeId::Okx;
        let symbol = instrument.symbol_for(exchange);
        let mut response = self
            .public_data
            .get_funding_rate(&symbol)
            .await
            .map_err(Error::from_okx)?;
        let item = response.drain(..).next().ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("OKX funding rate response is empty for {symbol}"),
        })?;

        okx_funding_rate_from_dto(exchange, instrument.clone(), Some(symbol), item)
    }

    pub(crate) async fn funding_rate_history(
        &self,
        query: FundingRateQuery,
    ) -> Result<Vec<FundingRate>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let before = query
            .before
            .as_deref()
            .map(|value| parse_i64_filter(exchange, "before", value))
            .transpose()?;
        let after = query
            .after
            .as_deref()
            .map(|value| parse_i64_filter(exchange, "after", value))
            .transpose()?;
        let limit = query.limit.map(i64::from);
        let response = self
            .public_data
            .get_funding_rate_history(&symbol, before, after, limit)
            .await
            .map_err(Error::from_okx)?;

        response
            .into_iter()
            .map(|item| {
                okx_funding_rate_from_history_dto(
                    exchange,
                    instrument.clone(),
                    Some(symbol.clone()),
                    item,
                )
            })
            .collect()
    }

    pub(crate) async fn mark_price(&self, instrument: &Instrument) -> Result<MarkPrice> {
        let exchange = ExchangeId::Okx;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .public_data
            .get_mark_price("SWAP", Some(&symbol), None, None)
            .await
            .map_err(Error::from_okx)?;
        let item = first_object_value(raw, exchange, "OKX mark price response")?;

        okx_mark_price_from_value(exchange, instrument.clone(), Some(symbol), item)
    }

    pub(crate) async fn open_interest(&self, instrument: &Instrument) -> Result<OpenInterest> {
        let exchange = ExchangeId::Okx;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .public_data
            .get_open_interest("SWAP", Some(&symbol), None, None)
            .await
            .map_err(Error::from_okx)?;
        let item = first_object_value(raw, exchange, "OKX open interest response")?;

        okx_open_interest_from_value(exchange, instrument.clone(), Some(symbol), item)
    }

    pub(crate) async fn long_short_ratio(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<LongShortRatio>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let limit = query.limit.map(|value| value.to_string());
        let begin = query.start_time.map(|value| value.to_string());
        let end = query.end_time.map(|value| value.to_string());
        let raw = self
            .big_data
            .get_long_short_account_ratio_contract_top_trader(
                &symbol,
                Some(&query.period),
                begin.as_deref(),
                end.as_deref(),
                limit.as_deref(),
            )
            .await
            .map_err(Error::from_okx)?;

        raw.into_iter()
            .map(|values| {
                okx_long_short_ratio_from_values(
                    exchange,
                    instrument.clone(),
                    symbol.clone(),
                    query.period.clone(),
                    values,
                )
            })
            .collect()
    }

    pub(crate) async fn top_trader_position_ratio(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<LongShortRatio>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let limit = query.limit.map(|value| value.to_string());
        let begin = query.start_time.map(|value| value.to_string());
        let end = query.end_time.map(|value| value.to_string());
        let raw = self
            .big_data
            .get_long_short_position_ratio_contract_top_trader(
                &symbol,
                Some(&query.period),
                begin.as_deref(),
                end.as_deref(),
                limit.as_deref(),
            )
            .await
            .map_err(Error::from_okx)?;

        raw.into_iter()
            .map(|values| {
                okx_long_short_ratio_from_values(
                    exchange,
                    instrument.clone(),
                    symbol.clone(),
                    query.period.clone(),
                    values,
                )
            })
            .collect()
    }

    pub(crate) async fn taker_buy_sell_volume(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<TakerBuySellVolume>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let limit = query.limit.map(|value| value.to_string());
        let begin = query.start_time.map(|value| value.to_string());
        let end = query.end_time.map(|value| value.to_string());
        let raw = self
            .big_data
            .get_taker_volume_contract(
                &symbol,
                Some(&query.period),
                None,
                begin.as_deref(),
                end.as_deref(),
                limit.as_deref(),
            )
            .await
            .map_err(Error::from_okx)?;

        raw.into_iter()
            .map(|values| {
                okx_taker_volume_from_values(
                    exchange,
                    instrument.clone(),
                    symbol.clone(),
                    query.period.clone(),
                    values,
                )
            })
            .collect()
    }
}
