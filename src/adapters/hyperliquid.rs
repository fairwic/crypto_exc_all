use super::hyperliquid_bills::hyperliquid_bill_from_value;
use super::hyperliquid_market::{
    asset_context_from_response, hyperliquid_candle_from_value, orderbook_levels,
    ticker_from_context, universe_and_contexts, universe_and_contexts_with_label,
};
use super::hyperliquid_orders::{
    hyperliquid_fill_from_value, hyperliquid_order_from_history_item,
    hyperliquid_order_from_status_response, hyperliquid_order_from_value,
};
use super::hyperliquid_spot::{
    spot_exchange_symbol_from_universe_item, spot_instrument_from_exchange_symbol,
    spot_instrument_from_symbol, spot_instrument_from_universe_item,
    spot_market_data_coin_from_meta,
};
use crate::account::{AccountBill, AccountBillQuery, AccountCapabilities, Balance};
use crate::config::HyperliquidExchangeConfig;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::fill::{Fill, FillListQuery};
use crate::instrument::{Instrument, MarketType};
use crate::market::{
    Candle, CandleQuery, FundingRate, FundingRateQuery, MarkPrice, OpenInterest, OrderBook,
    OrderBookQuery, Ticker,
};
use crate::order::{Order, OrderListQuery, OrderQuery};
use crate::position::Position;
use hyperliquid_rs::{Config as HyperliquidConfig, HyperliquidClient};
use serde_json::{Map, Value};

pub(crate) struct HyperliquidAdapter {
    client: HyperliquidClient,
    user_address: Option<String>,
}

impl HyperliquidAdapter {
    pub(crate) fn new(config: HyperliquidExchangeConfig) -> Result<Self> {
        let mut hyperliquid_config = HyperliquidConfig::from_env();
        if let Some(api_url) = config.api_url {
            hyperliquid_config.api_url = api_url;
        }
        if let Some(api_timeout_ms) = config.api_timeout_ms {
            hyperliquid_config.api_timeout_ms = api_timeout_ms;
        }
        if let Some(proxy_url) = config.proxy_url {
            hyperliquid_config.proxy_url = Some(proxy_url);
        }

        Ok(Self {
            client: HyperliquidClient::with_config(hyperliquid_config)
                .map_err(Error::from_hyperliquid)?,
            user_address: config.user_address,
        })
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        let exchange = ExchangeId::Hyperliquid;
        let symbol = instrument.symbol_for(exchange);
        let (ctx, instrument_type) = if matches!(
            instrument.market_type,
            MarketType::Spot | MarketType::Margin
        ) {
            let (exchange_symbol, ctx) = self.spot_asset_context(instrument).await?;
            return ticker_from_context(exchange, instrument.clone(), "spot", exchange_symbol, ctx);
        } else {
            let (_, ctx) = self.asset_context(&symbol).await?;
            (ctx, "perp")
        };

        ticker_from_context(exchange, instrument.clone(), instrument_type, symbol, ctx)
    }

    pub(crate) async fn tickers(&self, instrument_type: &str) -> Result<Vec<Ticker>> {
        if instrument_type.eq_ignore_ascii_case("spot") {
            return self.spot_tickers().await;
        }

        let exchange = ExchangeId::Hyperliquid;
        let raw = self
            .client
            .meta_and_asset_ctxs()
            .await
            .map_err(Error::from_hyperliquid)?;
        let (universe, contexts) = universe_and_contexts(&raw, exchange)?;
        let mut output = Vec::new();

        for (index, asset) in universe.iter().enumerate() {
            let asset_object = object_value(asset, exchange, "Hyperliquid universe item")?;
            let Some(coin) = string_field(asset_object, "name") else {
                continue;
            };
            let Some(ctx) = contexts.get(index).cloned() else {
                continue;
            };
            output.push(ticker_from_context(
                exchange,
                Instrument::perp(&coin, "USDC"),
                "perp",
                coin,
                ctx,
            )?);
        }

        Ok(output)
    }

    pub(crate) async fn orderbook(&self, query: OrderBookQuery) -> Result<OrderBook> {
        let exchange = ExchangeId::Hyperliquid;
        let coin = self.market_data_coin(&query.instrument).await?;
        let raw = self
            .client
            .l2_book(&coin)
            .await
            .map_err(Error::from_hyperliquid)?;
        let object = object_value(&raw, exchange, "Hyperliquid l2Book response")?;
        let levels = object
            .get("levels")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: "Hyperliquid l2Book response missing levels".to_string(),
            })?;
        let mut bids = orderbook_levels(levels.first(), exchange)?;
        let mut asks = orderbook_levels(levels.get(1), exchange)?;
        if let Some(limit) = query.limit {
            let limit = limit as usize;
            bids.truncate(limit);
            asks.truncate(limit);
        }

        Ok(OrderBook {
            exchange,
            instrument: query.instrument,
            exchange_symbol: coin,
            bids,
            asks,
            timestamp: u64_field(object, "time"),
            raw,
        })
    }

    pub(crate) async fn candles(&self, query: CandleQuery) -> Result<Vec<Candle>> {
        let exchange = ExchangeId::Hyperliquid;
        let coin = self.market_data_coin(&query.instrument).await?;
        let start_time = query.start_time.ok_or_else(|| Error::Adapter {
            exchange,
            message: "Hyperliquid candleSnapshot requires start_time".to_string(),
        })?;
        let end_time = query.end_time.ok_or_else(|| Error::Adapter {
            exchange,
            message: "Hyperliquid candleSnapshot requires end_time".to_string(),
        })?;
        let raw = self
            .client
            .candle_snapshot(&coin, &query.interval, start_time, end_time)
            .await
            .map_err(Error::from_hyperliquid)?;
        let items = array_value(&raw, exchange, "Hyperliquid candleSnapshot response")?;
        let limit = query.limit.unwrap_or(u32::MAX) as usize;

        items
            .iter()
            .take(limit)
            .map(|item| hyperliquid_candle_from_value(exchange, query.instrument.clone(), item))
            .collect()
    }

    pub(crate) async fn funding_rate(&self, instrument: &Instrument) -> Result<FundingRate> {
        let exchange = ExchangeId::Hyperliquid;
        let coin = instrument.symbol_for(exchange);
        let (_, ctx) = self.asset_context(&coin).await?;
        let object = object_value(&ctx, exchange, "Hyperliquid asset context")?;
        let predicted = self
            .client
            .predicted_fundings()
            .await
            .map_err(Error::from_hyperliquid)?;
        let (next_funding_rate, next_funding_time) =
            predicted_funding_for_coin(&predicted, exchange, &coin)?;

        Ok(FundingRate {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: coin,
            funding_rate: string_field(object, "funding").unwrap_or_default(),
            funding_time: None,
            next_funding_rate,
            next_funding_time,
            mark_price: string_field(object, "markPx"),
            raw: ctx,
        })
    }

    pub(crate) async fn funding_rate_history(
        &self,
        query: FundingRateQuery,
    ) -> Result<Vec<FundingRate>> {
        let exchange = ExchangeId::Hyperliquid;
        let coin = query.instrument.symbol_for(exchange);
        let start_time = query.start_time.ok_or_else(|| Error::Adapter {
            exchange,
            message: "Hyperliquid fundingHistory requires start_time".to_string(),
        })?;
        let raw = self
            .client
            .funding_history(&coin, start_time, query.end_time)
            .await
            .map_err(Error::from_hyperliquid)?;
        let items = array_value(&raw, exchange, "Hyperliquid fundingHistory response")?;
        let limit = query.limit.unwrap_or(u32::MAX) as usize;

        items
            .iter()
            .take(limit)
            .map(|item| {
                let object = object_value(item, exchange, "Hyperliquid fundingHistory item")?;
                Ok(FundingRate {
                    exchange,
                    instrument: query.instrument.clone(),
                    exchange_symbol: string_field(object, "coin").unwrap_or_else(|| coin.clone()),
                    funding_rate: string_field(object, "fundingRate").unwrap_or_default(),
                    funding_time: u64_field(object, "time"),
                    next_funding_rate: None,
                    next_funding_time: None,
                    mark_price: None,
                    raw: item.clone(),
                })
            })
            .collect()
    }

    pub(crate) async fn mark_price(&self, instrument: &Instrument) -> Result<MarkPrice> {
        let exchange = ExchangeId::Hyperliquid;
        let coin = instrument.symbol_for(exchange);
        let (_, ctx) = self.asset_context(&coin).await?;
        let object = object_value(&ctx, exchange, "Hyperliquid asset context")?;

        Ok(MarkPrice {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: coin,
            mark_price: string_field(object, "markPx").unwrap_or_default(),
            index_price: string_field(object, "oraclePx"),
            funding_rate: string_field(object, "funding"),
            next_funding_time: None,
            timestamp: None,
            raw: ctx,
        })
    }

    pub(crate) async fn open_interest(&self, instrument: &Instrument) -> Result<OpenInterest> {
        let exchange = ExchangeId::Hyperliquid;
        let coin = instrument.symbol_for(exchange);
        let (_, ctx) = self.asset_context(&coin).await?;
        let object = object_value(&ctx, exchange, "Hyperliquid asset context")?;

        Ok(OpenInterest {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: coin,
            open_interest: string_field(object, "openInterest").unwrap_or_default(),
            open_interest_value: None,
            timestamp: None,
            raw: ctx,
        })
    }

    pub(crate) async fn balances(&self) -> Result<Vec<Balance>> {
        let exchange = ExchangeId::Hyperliquid;
        let raw = self.clearinghouse_state("balances").await?;
        let object = object_value(&raw, exchange, "Hyperliquid clearinghouseState response")?;
        let summary = object
            .get("marginSummary")
            .and_then(Value::as_object)
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: "Hyperliquid clearinghouseState missing marginSummary".to_string(),
            })?;

        let mut balances = vec![Balance {
            exchange,
            asset: "USDC".to_string(),
            total: string_field(summary, "accountValue").unwrap_or_default(),
            available: string_field(object, "withdrawable").unwrap_or_default(),
            frozen: string_field(summary, "totalMarginUsed"),
            raw,
        }];

        let spot_raw = self.spot_clearinghouse_state("spot balances").await?;
        let spot_object = object_value(
            &spot_raw,
            exchange,
            "Hyperliquid spotClearinghouseState response",
        )?;
        let spot_items = spot_object
            .get("balances")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: "Hyperliquid spotClearinghouseState missing balances".to_string(),
            })?;
        for item in spot_items {
            balances.push(hyperliquid_spot_balance_from_value(exchange, item)?);
        }

        Ok(balances)
    }

    pub(crate) async fn account_bills(&self, query: AccountBillQuery) -> Result<Vec<AccountBill>> {
        let exchange = ExchangeId::Hyperliquid;
        let start_time = query.start_time.ok_or_else(|| Error::Adapter {
            exchange,
            message: "Hyperliquid account bills require start_time".to_string(),
        })?;
        let user = self.user_address("account bills")?;
        let bill_type = query
            .bill_type
            .as_deref()
            .map(|value| value.trim().to_ascii_lowercase());

        let mut bills = match bill_type.as_deref() {
            None | Some("") => {
                let mut items = self
                    .user_non_funding_bills(user, start_time, query.end_time)
                    .await?;
                items.extend(
                    self.user_funding_bills(user, start_time, query.end_time)
                        .await?,
                );
                items
            }
            Some("funding") => {
                self.user_funding_bills(user, start_time, query.end_time)
                    .await?
            }
            Some("non_funding" | "nonfunding" | "ledger" | "non_funding_ledger") => {
                self.user_non_funding_bills(user, start_time, query.end_time)
                    .await?
            }
            Some(_) => {
                return Err(Error::Unsupported {
                    exchange,
                    capability: "Hyperliquid account bill_type filter",
                });
            }
        };

        bills.sort_by_key(|bill| bill.timestamp.unwrap_or(u64::MAX));
        bills.truncate(query.limit.unwrap_or(u32::MAX) as usize);
        Ok(bills)
    }

    async fn user_non_funding_bills(
        &self,
        user: &str,
        start_time: u64,
        end_time: Option<u64>,
    ) -> Result<Vec<AccountBill>> {
        let exchange = ExchangeId::Hyperliquid;
        let raw = self
            .client
            .user_non_funding_ledger_updates(user, start_time, end_time)
            .await
            .map_err(Error::from_hyperliquid)?;
        let items = array_value(
            &raw,
            exchange,
            "Hyperliquid userNonFundingLedgerUpdates response",
        )?;

        items
            .iter()
            .map(|item| hyperliquid_bill_from_value(exchange, item))
            .collect()
    }

    async fn user_funding_bills(
        &self,
        user: &str,
        start_time: u64,
        end_time: Option<u64>,
    ) -> Result<Vec<AccountBill>> {
        let exchange = ExchangeId::Hyperliquid;
        let raw = self
            .client
            .user_funding(user, start_time, end_time)
            .await
            .map_err(Error::from_hyperliquid)?;
        let items = array_value(&raw, exchange, "Hyperliquid userFunding response")?;

        items
            .iter()
            .map(|item| hyperliquid_bill_from_value(exchange, item))
            .collect()
    }

    pub(crate) fn account_capabilities(&self) -> AccountCapabilities {
        AccountCapabilities {
            set_leverage: false,
            set_position_mode: false,
            set_symbol_margin_mode: false,
            order_level_margin_mode: false,
        }
    }

    pub(crate) async fn positions(&self, instrument: Option<&Instrument>) -> Result<Vec<Position>> {
        let exchange = ExchangeId::Hyperliquid;
        let raw = self.clearinghouse_state("positions").await?;
        let object = object_value(&raw, exchange, "Hyperliquid clearinghouseState response")?;
        let items = object
            .get("assetPositions")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: "Hyperliquid clearinghouseState missing assetPositions".to_string(),
            })?;
        let filter_symbol = instrument.map(|value| value.symbol_for(exchange));
        let mut output = Vec::new();

        for item in items {
            let position = item
                .get("position")
                .and_then(Value::as_object)
                .ok_or_else(|| Error::Adapter {
                    exchange,
                    message: "Hyperliquid assetPosition missing position".to_string(),
                })?;
            let coin = string_field(position, "coin").unwrap_or_default();
            if filter_symbol
                .as_deref()
                .is_some_and(|symbol| symbol != coin)
            {
                continue;
            }
            let leverage = position.get("leverage").and_then(Value::as_object);
            output.push(Position {
                exchange,
                instrument: instrument
                    .cloned()
                    .unwrap_or_else(|| Instrument::perp(&coin, "USDC")),
                exchange_symbol: coin,
                side: string_field(position, "szi").and_then(|size| position_side(&size)),
                size: string_field(position, "szi").unwrap_or_default(),
                entry_price: string_field(position, "entryPx"),
                mark_price: None,
                unrealized_pnl: string_field(position, "unrealizedPnl"),
                leverage: leverage.and_then(|object| string_field(object, "value")),
                margin_mode: leverage.and_then(|object| string_field(object, "type")),
                liquidation_price: string_field(position, "liquidationPx"),
                raw: item.clone(),
            });
        }

        Ok(output)
    }

    pub(crate) async fn order(&self, query: OrderQuery) -> Result<Order> {
        let exchange = ExchangeId::Hyperliquid;
        let order_id = query.order_id.as_deref().ok_or_else(|| Error::Adapter {
            exchange,
            message: "Hyperliquid orderStatus requires order_id".to_string(),
        })?;
        let user = self.user_address("order detail")?;
        let raw = self
            .client
            .order_status(user, order_id)
            .await
            .map_err(Error::from_hyperliquid)?;

        hyperliquid_order_from_status_response(exchange, query.instrument, raw)
    }

    pub(crate) async fn open_orders(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        let exchange = ExchangeId::Hyperliquid;
        let user = self.user_address("open orders")?;
        let raw = self
            .client
            .frontend_open_orders(user, None)
            .await
            .map_err(Error::from_hyperliquid)?;
        let items = array_value(&raw, exchange, "Hyperliquid frontendOpenOrders response")?;
        let filter_symbol = self.filter_coin(query.instrument.clone()).await?;
        let limit = query.limit.unwrap_or(u32::MAX) as usize;
        let mut output = Vec::new();
        let mut spot_meta = None;

        for item in items {
            let mut order = hyperliquid_order_from_value(
                exchange,
                query.instrument.clone(),
                item.clone(),
                Some("open".to_string()),
            )?;
            order.instrument = self
                .instrument_for_coin(
                    query.instrument.clone(),
                    &order.exchange_symbol,
                    &mut spot_meta,
                )
                .await?;
            if filter_symbol
                .as_deref()
                .is_some_and(|symbol| symbol != order.exchange_symbol)
            {
                continue;
            }
            output.push(order);
            if output.len() >= limit {
                break;
            }
        }

        Ok(output)
    }

    pub(crate) async fn order_history(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        let exchange = ExchangeId::Hyperliquid;
        let user = self.user_address("order history")?;
        let raw = self
            .client
            .historical_orders(user)
            .await
            .map_err(Error::from_hyperliquid)?;
        let items = array_value(&raw, exchange, "Hyperliquid historicalOrders response")?;
        let filter_symbol = self.filter_coin(query.instrument.clone()).await?;
        let limit = query.limit.unwrap_or(u32::MAX) as usize;
        let mut output = Vec::new();
        let mut spot_meta = None;

        for item in items {
            let mut order =
                hyperliquid_order_from_history_item(exchange, query.instrument.clone(), item)?;
            order.instrument = self
                .instrument_for_coin(
                    query.instrument.clone(),
                    &order.exchange_symbol,
                    &mut spot_meta,
                )
                .await?;
            if filter_symbol
                .as_deref()
                .is_some_and(|symbol| symbol != order.exchange_symbol)
            {
                continue;
            }
            output.push(order);
            if output.len() >= limit {
                break;
            }
        }

        Ok(output)
    }

    pub(crate) async fn fills(&self, query: FillListQuery) -> Result<Vec<Fill>> {
        let exchange = ExchangeId::Hyperliquid;
        let user = self.user_address("fills")?;
        let raw = if let Some(start_time) = query.start_time {
            self.client
                .user_fills_by_time(user, start_time, query.end_time)
                .await
        } else {
            self.client.user_fills(user).await
        }
        .map_err(Error::from_hyperliquid)?;
        let items = array_value(&raw, exchange, "Hyperliquid fills response")?;
        let filter_symbol = self.filter_coin(query.instrument.clone()).await?;
        let limit = query.limit.unwrap_or(u32::MAX) as usize;
        let mut output = Vec::new();
        let mut spot_meta = None;

        for item in items {
            let mut fill = hyperliquid_fill_from_value(exchange, query.instrument.clone(), item)?;
            fill.instrument = self
                .instrument_for_coin(
                    query.instrument.clone(),
                    &fill.exchange_symbol,
                    &mut spot_meta,
                )
                .await?;
            if filter_symbol
                .as_deref()
                .is_some_and(|symbol| symbol != fill.exchange_symbol)
            {
                continue;
            }
            if query
                .order_id
                .as_deref()
                .is_some_and(|order_id| fill.order_id.as_deref() != Some(order_id))
            {
                continue;
            }
            output.push(fill);
            if output.len() >= limit {
                break;
            }
        }

        Ok(output)
    }

    async fn asset_context(&self, coin: &str) -> Result<(Value, Value)> {
        let exchange = ExchangeId::Hyperliquid;
        let raw = self
            .client
            .meta_and_asset_ctxs()
            .await
            .map_err(Error::from_hyperliquid)?;
        asset_context_from_response(&raw, exchange, coin, "Hyperliquid metaAndAssetCtxs")
    }

    async fn market_data_coin(&self, instrument: &Instrument) -> Result<String> {
        if !matches!(
            instrument.market_type,
            MarketType::Spot | MarketType::Margin
        ) {
            return Ok(instrument.symbol_for(ExchangeId::Hyperliquid));
        }
        let fallback = instrument.symbol_for(ExchangeId::Hyperliquid);
        if fallback == "PURR/USDC" {
            return Ok(fallback);
        }
        let raw = self
            .client
            .spot_meta()
            .await
            .map_err(Error::from_hyperliquid)?;
        spot_market_data_coin_from_meta(&raw, instrument, &fallback)
    }

    async fn filter_coin(&self, instrument: Option<Instrument>) -> Result<Option<String>> {
        match instrument {
            Some(instrument) => self.market_data_coin(&instrument).await.map(Some),
            None => Ok(None),
        }
    }

    async fn instrument_for_coin(
        &self,
        instrument: Option<Instrument>,
        coin: &str,
        spot_meta: &mut Option<Value>,
    ) -> Result<Instrument> {
        if let Some(instrument) = instrument {
            return Ok(instrument);
        }
        if coin.contains('/') {
            return Ok(spot_instrument_from_symbol(coin));
        }
        if !coin.starts_with('@') {
            return Ok(Instrument::perp(coin, "USDC"));
        }
        if spot_meta.is_none() {
            *spot_meta = Some(
                self.client
                    .spot_meta()
                    .await
                    .map_err(Error::from_hyperliquid)?,
            );
        }
        let raw = spot_meta.as_ref().expect("spot_meta is initialized above");
        spot_instrument_from_exchange_symbol(raw, coin)
    }

    async fn spot_tickers(&self) -> Result<Vec<Ticker>> {
        let exchange = ExchangeId::Hyperliquid;
        let raw = self
            .client
            .spot_meta_and_asset_ctxs()
            .await
            .map_err(Error::from_hyperliquid)?;
        let meta = raw
            .as_array()
            .and_then(|items| items.first())
            .and_then(Value::as_object)
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: "Hyperliquid spotMetaAndAssetCtxs missing meta".to_string(),
            })?;
        let tokens = meta
            .get("tokens")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: "Hyperliquid spotMetaAndAssetCtxs missing tokens".to_string(),
            })?;
        let (universe, contexts) =
            universe_and_contexts_with_label(&raw, exchange, "Hyperliquid spotMetaAndAssetCtxs")?;
        let mut output = Vec::new();

        for (index, asset) in universe.iter().enumerate() {
            let Some(symbol) = spot_exchange_symbol_from_universe_item(asset) else {
                continue;
            };
            let Some(ctx) = contexts.get(index).cloned() else {
                continue;
            };
            let instrument = spot_instrument_from_universe_item(tokens, asset)
                .unwrap_or_else(|| spot_instrument_from_symbol(&symbol));
            output.push(ticker_from_context(
                exchange, instrument, "spot", symbol, ctx,
            )?);
        }

        Ok(output)
    }

    async fn spot_asset_context(&self, instrument: &Instrument) -> Result<(String, Value)> {
        let exchange = ExchangeId::Hyperliquid;
        let raw = self
            .client
            .spot_meta_and_asset_ctxs()
            .await
            .map_err(Error::from_hyperliquid)?;
        let meta = raw
            .as_array()
            .and_then(|items| items.first())
            .and_then(Value::as_object)
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: "Hyperliquid spotMetaAndAssetCtxs missing meta".to_string(),
            })?;
        let tokens = meta
            .get("tokens")
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: "Hyperliquid spotMetaAndAssetCtxs missing tokens".to_string(),
            })?;
        let (universe, contexts) =
            universe_and_contexts_with_label(&raw, exchange, "Hyperliquid spotMetaAndAssetCtxs")?;

        for (index, asset) in universe.iter().enumerate() {
            if spot_instrument_from_universe_item(tokens, asset).as_ref() != Some(instrument) {
                continue;
            }
            let exchange_symbol =
                spot_exchange_symbol_from_universe_item(asset).ok_or_else(|| Error::Adapter {
                    exchange,
                    message: format!(
                        "Hyperliquid spotMetaAndAssetCtxs asset missing exchange symbol: {}",
                        instrument.symbol_for(exchange)
                    ),
                })?;
            let ctx = contexts.get(index).cloned().ok_or_else(|| Error::Adapter {
                exchange,
                message: format!(
                    "Hyperliquid spotMetaAndAssetCtxs context missing asset: {exchange_symbol}"
                ),
            })?;
            return Ok((exchange_symbol, ctx));
        }

        Err(Error::Adapter {
            exchange,
            message: format!(
                "Hyperliquid spotMetaAndAssetCtxs asset not found: {}",
                instrument.symbol_for(exchange)
            ),
        })
    }

    async fn clearinghouse_state(&self, capability: &'static str) -> Result<Value> {
        let user = self.user_address(capability)?;
        self.client
            .clearinghouse_state(user)
            .await
            .map_err(Error::from_hyperliquid)
    }

    async fn spot_clearinghouse_state(&self, capability: &'static str) -> Result<Value> {
        let user = self.user_address(capability)?;
        self.client
            .spot_clearinghouse_state(user)
            .await
            .map_err(Error::from_hyperliquid)
    }

    fn user_address(&self, capability: &'static str) -> Result<&str> {
        self.user_address.as_deref().ok_or_else(|| {
            Error::Config(format!(
                "HYPERLIQUID_USER_ADDRESS is required for Hyperliquid {capability}"
            ))
        })
    }
}

fn hyperliquid_spot_balance_from_value(exchange: ExchangeId, raw: &Value) -> Result<Balance> {
    let object = object_value(raw, exchange, "Hyperliquid spot balance item")?;
    let total = string_field(object, "total").unwrap_or_default();

    Ok(Balance {
        exchange,
        asset: string_field(object, "coin").unwrap_or_default(),
        total: total.clone(),
        available: total,
        frozen: string_field(object, "hold"),
        raw: raw.clone(),
    })
}

fn position_side(size: &str) -> Option<String> {
    if size.trim_start().starts_with('-') {
        Some("short".to_string())
    } else if size.trim().is_empty() || size.trim() == "0" {
        None
    } else {
        Some("long".to_string())
    }
}

fn predicted_funding_for_coin(
    raw: &Value,
    exchange: ExchangeId,
    coin: &str,
) -> Result<(Option<String>, Option<u64>)> {
    let items = array_value(raw, exchange, "Hyperliquid predictedFundings response")?;
    for item in items {
        let item_values = item.as_array().ok_or_else(|| Error::Adapter {
            exchange,
            message: "Hyperliquid predictedFundings item is not an array".to_string(),
        })?;
        if item_values.first().and_then(value_to_string).as_deref() != Some(coin) {
            continue;
        }
        let venues = item_values
            .get(1)
            .and_then(Value::as_array)
            .ok_or_else(|| Error::Adapter {
                exchange,
                message: format!("Hyperliquid predictedFundings missing venues for {coin}"),
            })?;
        for venue in venues {
            let venue_values = venue.as_array().ok_or_else(|| Error::Adapter {
                exchange,
                message: "Hyperliquid predictedFundings venue is not an array".to_string(),
            })?;
            if venue_values.first().and_then(value_to_string).as_deref() != Some("HlPerp") {
                continue;
            }
            let venue_object = venue_values
                .get(1)
                .and_then(Value::as_object)
                .ok_or_else(|| Error::Adapter {
                    exchange,
                    message: "Hyperliquid predictedFundings venue data is not an object"
                        .to_string(),
                })?;
            return Ok((
                string_field(venue_object, "fundingRate"),
                u64_field(venue_object, "nextFundingTime"),
            ));
        }
    }

    Ok((None, None))
}

fn object_value<'a>(
    value: &'a Value,
    exchange: ExchangeId,
    message: &'static str,
) -> Result<&'a Map<String, Value>> {
    value.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: format!("{message} is not an object"),
    })
}

fn array_value<'a>(
    value: &'a Value,
    exchange: ExchangeId,
    message: &'static str,
) -> Result<&'a Vec<Value>> {
    value.as_array().ok_or_else(|| Error::Adapter {
        exchange,
        message: format!("{message} is not an array"),
    })
}

fn string_field(object: &Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(value_to_string)
}

fn u64_field(object: &Map<String, Value>, key: &str) -> Option<u64> {
    object.get(key).and_then(|value| match value {
        Value::Number(number) => number.as_u64(),
        Value::String(value) => value.parse::<u64>().ok(),
        _ => None,
    })
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}
