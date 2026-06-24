use crate::account::{
    AccountBill, AccountBillQuery, AccountCapabilities, Balance, EnsureOrderMarginModeRequest,
    EnsureOrderMarginModeResult, LeverageSetting, PositionModeSetting, SetLeverageRequest,
    SetPositionModeRequest, SetSymbolMarginModeRequest, SymbolMarginModeSetting,
};
use crate::config::BybitExchangeConfig;
use crate::error::{Error, Result};
use crate::exchange::ExchangeId;
use crate::fill::{Fill, FillListQuery};
use crate::instrument::Instrument;
use crate::market::{
    Candle, CandleQuery, FundingRate, FundingRateQuery, LongShortRatio, MarkPrice,
    MarketStatsQuery, OpenInterest, OrderBook, OrderBookLevel, OrderBookQuery, TakerBuySellVolume,
    Ticker,
};
use crate::order::{Order, OrderListQuery, OrderQuery};
use crate::platform::{PlatformEvent, PlatformEventQuery};
use crate::position::Position;
use crate::trade::{
    CancelOrderRequest, OrderAck, OrderSide, OrderType, PlaceOrderRequest, TimeInForce,
};
use bybit_rs::{
    BybitClient, BybitDepositRecordRequest, BybitTransferRecordRequest,
    BybitWithdrawalRecordRequest, CancelOrderRequest as BybitCancelOrderRequest,
    Config as BybitConfig, Credentials as BybitCredentials, OrderRequest as BybitOrderRequest,
    OrderStatusRequest as BybitOrderStatusRequest, PositionListRequest as BybitPositionListRequest,
};
use serde_json::Value;

const DEFAULT_CATEGORY: &str = "linear";

macro_rules! unsupported_methods {
    ($($name:ident (&self $(, $arg:ident : $arg_ty:ty)*) -> $ret:ty, $capability:literal;)+) => {
        $(
            pub(crate) async fn $name(&self, $($arg: $arg_ty),*) -> Result<$ret> {
                Err(Error::Unsupported {
                    exchange: ExchangeId::Bybit,
                    capability: $capability,
                })
            }
        )+
    };
}

pub(crate) struct BybitAdapter {
    client: BybitClient,
    category: String,
}

impl BybitAdapter {
    pub(crate) fn new(config: BybitExchangeConfig) -> Result<Self> {
        let mut bybit_config = BybitConfig::from_env();
        if let Some(api_url) = config.api_url {
            bybit_config.api_url = api_url;
        }
        if let Some(api_timeout_ms) = config.api_timeout_ms {
            bybit_config.api_timeout_ms = api_timeout_ms;
        }
        if let Some(recv_window_ms) = config.recv_window_ms {
            bybit_config.recv_window_ms = recv_window_ms;
        }
        if let Some(proxy_url) = config.proxy_url {
            bybit_config.proxy_url = Some(proxy_url);
        }

        Ok(Self {
            client: BybitClient::with_config(
                Some(BybitCredentials::new(config.api_key, config.api_secret)),
                bybit_config,
            )
            .map_err(Error::from_bybit)?,
            category: config
                .category
                .unwrap_or_else(|| DEFAULT_CATEGORY.to_string()),
        })
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        let exchange = ExchangeId::Bybit;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .client
            .ticker(&self.category, &symbol)
            .await
            .map_err(Error::from_bybit)?;
        let item = first_list_item(&raw, exchange, "Bybit ticker response")?;

        Ok(Ticker {
            exchange,
            instrument: instrument.clone(),
            instrument_type: Some(self.category.clone()),
            exchange_symbol: symbol,
            last_price: string_field(item, "lastPrice").unwrap_or_default(),
            last_size: None,
            bid_price: string_field(item, "bid1Price"),
            bid_size: string_field(item, "bid1Size"),
            ask_price: string_field(item, "ask1Price"),
            ask_size: string_field(item, "ask1Size"),
            open_24h: string_field(item, "prevPrice24h"),
            high_24h: string_field(item, "highPrice24h"),
            low_24h: string_field(item, "lowPrice24h"),
            volume_24h: string_field(item, "turnover24h")
                .or_else(|| string_field(item, "volume24h")),
            base_volume_24h: string_field(item, "volume24h"),
            quote_volume_24h: string_field(item, "turnover24h"),
            sod_utc0: None,
            sod_utc8: None,
            timestamp: None,
            raw,
        })
    }

    pub(crate) async fn tickers(&self, _instrument_type: &str) -> Result<Vec<Ticker>> {
        Err(Error::Unsupported {
            exchange: ExchangeId::Bybit,
            capability: "market tickers",
        })
    }

    pub(crate) async fn orderbook(&self, query: OrderBookQuery) -> Result<OrderBook> {
        let exchange = ExchangeId::Bybit;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .client
            .orderbook(&self.category, &symbol, query.limit)
            .await
            .map_err(Error::from_bybit)?;

        Ok(OrderBook {
            exchange,
            instrument,
            exchange_symbol: symbol,
            bids: bybit_levels(raw.get("b")),
            asks: bybit_levels(raw.get("a")),
            timestamp: raw.get("ts").and_then(value_u64),
            raw,
        })
    }

    pub(crate) async fn candles(&self, query: CandleQuery) -> Result<Vec<Candle>> {
        let exchange = ExchangeId::Bybit;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let interval = bybit_candle_interval(&query.interval);
        let raw = self
            .client
            .kline(
                &self.category,
                &symbol,
                &interval,
                query.limit,
                query.start_time,
                query.end_time,
            )
            .await
            .map_err(Error::from_bybit)?;
        let rows = raw
            .get("list")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(rows
            .into_iter()
            .filter_map(|row| bybit_candle(exchange, &instrument, &symbol, row))
            .collect())
    }

    pub(crate) async fn positions(&self, instrument: Option<&Instrument>) -> Result<Vec<Position>> {
        let exchange = ExchangeId::Bybit;
        let symbol = instrument.map(|instrument| instrument.symbol_for(exchange));
        let raw = self
            .client
            .positions(&BybitPositionListRequest {
                category: self.category.clone(),
                symbol: symbol.clone(),
            })
            .await
            .map_err(Error::from_bybit)?;
        let list = raw
            .get("list")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(list
            .into_iter()
            .map(|item| {
                let exchange_symbol = string_field(&item, "symbol").unwrap_or_default();
                let instrument = instrument
                    .cloned()
                    .unwrap_or_else(|| instrument_from_symbol(&exchange_symbol));
                Position {
                    exchange,
                    instrument,
                    exchange_symbol,
                    side: string_field(&item, "side"),
                    size: string_field(&item, "size").unwrap_or_default(),
                    entry_price: string_field(&item, "avgPrice"),
                    mark_price: string_field(&item, "markPrice"),
                    unrealized_pnl: string_field(&item, "unrealisedPnl"),
                    leverage: string_field(&item, "leverage"),
                    margin_mode: string_field(&item, "tradeMode"),
                    liquidation_price: string_field(&item, "liqPrice"),
                    raw: item,
                }
            })
            .collect())
    }

    pub(crate) async fn place_order(&self, request: PlaceOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Bybit;
        if request.attached_stop_loss_price.is_some() {
            return Err(Error::Unsupported {
                exchange,
                capability: "attached stop loss on place_order",
            });
        }

        let symbol = request.instrument.symbol_for(exchange);
        let raw = self
            .client
            .place_order(&BybitOrderRequest {
                category: self.category.clone(),
                symbol: symbol.clone(),
                side: bybit_side(request.side).to_string(),
                order_type: bybit_order_type(request.order_type).to_string(),
                qty: request.size,
                price: request.price,
                time_in_force: request.time_in_force.map(bybit_tif).map(ToOwned::to_owned),
                order_link_id: request.client_order_id.clone(),
                reduce_only: request.reduce_only,
            })
            .await
            .map_err(Error::from_bybit)?;
        Ok(OrderAck {
            exchange,
            instrument: request.instrument,
            exchange_symbol: symbol,
            order_id: string_field(&raw, "orderId"),
            client_order_id: string_field(&raw, "orderLinkId").or(request.client_order_id),
            status: None,
            raw,
        })
    }

    pub(crate) async fn cancel_order(&self, request: CancelOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Bybit;
        let symbol = request.instrument.symbol_for(exchange);
        let raw = self
            .client
            .cancel_order(&BybitCancelOrderRequest {
                category: self.category.clone(),
                symbol: symbol.clone(),
                order_id: request.order_id.clone(),
                order_link_id: request.client_order_id.clone(),
            })
            .await
            .map_err(Error::from_bybit)?;
        Ok(OrderAck {
            exchange,
            instrument: request.instrument,
            exchange_symbol: symbol,
            order_id: string_field(&raw, "orderId").or(request.order_id),
            client_order_id: string_field(&raw, "orderLinkId").or(request.client_order_id),
            status: Some("cancelled".to_string()),
            raw,
        })
    }

    pub(crate) async fn order(&self, query: OrderQuery) -> Result<Order> {
        let exchange = ExchangeId::Bybit;
        let symbol = query.instrument.symbol_for(exchange);
        let raw = self
            .client
            .order_status(&BybitOrderStatusRequest {
                category: self.category.clone(),
                symbol: symbol.clone(),
                order_id: query.order_id.clone(),
                order_link_id: query.client_order_id.clone(),
            })
            .await
            .map_err(Error::from_bybit)?;
        let item = first_list_item(&raw, exchange, "Bybit order response")?;
        Ok(order_from_value(
            exchange,
            query.instrument,
            symbol,
            item.clone(),
        ))
    }

    pub(crate) fn account_capabilities(&self) -> AccountCapabilities {
        AccountCapabilities {
            set_leverage: false,
            set_position_mode: false,
            set_symbol_margin_mode: false,
            order_level_margin_mode: false,
        }
    }

    pub(crate) async fn platform_system_status(
        &self,
        query: PlatformEventQuery,
    ) -> Result<Vec<PlatformEvent>> {
        if query.locale.is_some()
            || query.event_type.is_some()
            || query.tag.is_some()
            || query.page.is_some()
            || query.limit.is_some()
        {
            return Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "platform system status announcement filters",
            });
        }

        let raw = self
            .client
            .system_status(query.id.as_deref(), query.state.as_deref())
            .await
            .map_err(Error::from_bybit)?;
        Ok(bybit_platform_items(raw)
            .into_iter()
            .map(bybit_system_status_event)
            .collect())
    }

    pub(crate) async fn platform_announcements(
        &self,
        query: PlatformEventQuery,
    ) -> Result<Vec<PlatformEvent>> {
        if query.id.is_some() || query.state.is_some() {
            return Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "platform announcements status filters",
            });
        }

        let locale = query.locale.as_deref().unwrap_or("en-US");
        let raw = self
            .client
            .announcements(
                locale,
                query.event_type.as_deref(),
                query.tag.as_deref(),
                query.page,
                query.limit,
            )
            .await
            .map_err(Error::from_bybit)?;
        Ok(bybit_platform_items(raw)
            .into_iter()
            .map(bybit_announcement_event)
            .collect())
    }

    pub(crate) async fn account_bills(&self, query: AccountBillQuery) -> Result<Vec<AccountBill>> {
        if query.instrument.is_some() || query.archive {
            return Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "account bills instrument/archive filter",
            });
        }

        let request_kind = BybitAccountBillKind::from_query(query.bill_type.as_deref())?;
        let mut output = Vec::new();

        if request_kind.includes_transfer() {
            let raw = self
                .client
                .internal_transfer_records(bybit_transfer_request(&query))
                .await
                .map_err(Error::from_bybit)?;
            output.extend(bybit_transfer_bills(raw)?);
        }
        if request_kind.includes_deposit() {
            let raw = self
                .client
                .deposit_records(bybit_deposit_request(&query))
                .await
                .map_err(Error::from_bybit)?;
            output.extend(bybit_deposit_bills(raw)?);
        }
        if request_kind.includes_withdrawal() {
            let raw = self
                .client
                .withdrawal_records(bybit_withdrawal_request(&query))
                .await
                .map_err(Error::from_bybit)?;
            output.extend(bybit_withdrawal_bills(raw)?);
        }

        Ok(output)
    }

    pub(crate) async fn funding_rate(&self, instrument: &Instrument) -> Result<FundingRate> {
        let exchange = ExchangeId::Bybit;
        let symbol = instrument.symbol_for(exchange);
        let item = self.ticker_item(&symbol).await?;
        Ok(FundingRate {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: symbol,
            funding_rate: string_field(&item, "fundingRate").unwrap_or_default(),
            funding_time: None,
            next_funding_rate: None,
            next_funding_time: item.get("nextFundingTime").and_then(value_u64),
            mark_price: string_field(&item, "markPrice"),
            raw: item,
        })
    }

    pub(crate) async fn funding_rate_history(
        &self,
        query: FundingRateQuery,
    ) -> Result<Vec<FundingRate>> {
        if query.after.is_some() || query.before.is_some() {
            return Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "funding rate history cursor",
            });
        }

        let exchange = ExchangeId::Bybit;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .client
            .funding_rate_history(
                &self.category,
                &symbol,
                query.start_time,
                query.end_time,
                query.limit,
            )
            .await
            .map_err(Error::from_bybit)?;
        let rows = raw
            .get("list")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(rows
            .into_iter()
            .map(|row| FundingRate {
                exchange,
                instrument: instrument.clone(),
                exchange_symbol: string_field(&row, "symbol").unwrap_or_else(|| symbol.clone()),
                funding_rate: string_field(&row, "fundingRate").unwrap_or_default(),
                funding_time: row.get("fundingRateTimestamp").and_then(value_u64),
                next_funding_rate: None,
                next_funding_time: None,
                mark_price: None,
                raw: row,
            })
            .collect())
    }

    pub(crate) async fn mark_price(&self, instrument: &Instrument) -> Result<MarkPrice> {
        let exchange = ExchangeId::Bybit;
        let symbol = instrument.symbol_for(exchange);
        let item = self.ticker_item(&symbol).await?;
        Ok(MarkPrice {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: symbol,
            mark_price: string_field(&item, "markPrice").unwrap_or_default(),
            index_price: string_field(&item, "indexPrice"),
            funding_rate: string_field(&item, "fundingRate"),
            next_funding_time: item.get("nextFundingTime").and_then(value_u64),
            timestamp: None,
            raw: item,
        })
    }

    pub(crate) async fn open_interest(&self, instrument: &Instrument) -> Result<OpenInterest> {
        let exchange = ExchangeId::Bybit;
        let symbol = instrument.symbol_for(exchange);
        let raw = self
            .client
            .open_interest(&self.category, &symbol, "5min", None, None, None, None)
            .await
            .map_err(Error::from_bybit)?;
        let item = first_list_item(&raw, exchange, "Bybit open interest response")?;
        Ok(OpenInterest {
            exchange,
            instrument: instrument.clone(),
            exchange_symbol: symbol,
            open_interest: string_field(item, "openInterest").unwrap_or_default(),
            open_interest_value: None,
            timestamp: item.get("timestamp").and_then(value_u64),
            raw,
        })
    }

    pub(crate) async fn open_interest_history(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<OpenInterest>> {
        let exchange = ExchangeId::Bybit;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let period = bybit_stats_period(&query.period);
        let raw = self
            .client
            .open_interest(
                &self.category,
                &symbol,
                &period,
                query.start_time,
                query.end_time,
                query.limit,
                None,
            )
            .await
            .map_err(Error::from_bybit)?;
        let rows = raw
            .get("list")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        Ok(rows
            .into_iter()
            .map(|row| OpenInterest {
                exchange,
                instrument: instrument.clone(),
                exchange_symbol: string_field(&row, "symbol").unwrap_or_else(|| symbol.clone()),
                open_interest: string_field(&row, "openInterest").unwrap_or_default(),
                open_interest_value: None,
                timestamp: row.get("timestamp").and_then(value_u64),
                raw: row,
            })
            .collect())
    }

    pub(crate) async fn long_short_ratio(
        &self,
        query: MarketStatsQuery,
    ) -> Result<Vec<LongShortRatio>> {
        let exchange = ExchangeId::Bybit;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        let period = bybit_stats_period(&query.period);
        let raw = self
            .client
            .long_short_ratio(
                &self.category,
                &symbol,
                &period,
                query.start_time,
                query.end_time,
                query.limit,
                None,
            )
            .await
            .map_err(Error::from_bybit)?;
        let rows = raw
            .get("list")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(rows
            .into_iter()
            .map(|row| {
                let long_ratio = string_field(&row, "buyRatio");
                let short_ratio = string_field(&row, "sellRatio");
                LongShortRatio {
                    exchange,
                    instrument: instrument.clone(),
                    exchange_symbol: string_field(&row, "symbol").unwrap_or_else(|| symbol.clone()),
                    period: query.period.clone(),
                    ratio: string_field(&row, "ratio")
                        .or_else(|| ratio_from_sides(long_ratio.as_deref(), short_ratio.as_deref()))
                        .unwrap_or_default(),
                    long_ratio,
                    short_ratio,
                    timestamp: row.get("timestamp").and_then(value_u64),
                    raw: row,
                }
            })
            .collect())
    }

    unsupported_methods! {
        taker_buy_sell_volume(&self, _query: MarketStatsQuery) -> Vec<TakerBuySellVolume>, "taker buy sell volume";
        balances(&self) -> Vec<Balance>, "balances";
        set_leverage(&self, _request: SetLeverageRequest) -> LeverageSetting, "set leverage";
        set_position_mode(&self, _request: SetPositionModeRequest) -> PositionModeSetting, "set position mode";
        set_symbol_margin_mode(&self, _request: SetSymbolMarginModeRequest) -> SymbolMarginModeSetting, "set symbol margin mode";
        ensure_order_margin_mode(&self, _request: EnsureOrderMarginModeRequest) -> EnsureOrderMarginModeResult, "ensure order margin mode";
        open_orders(&self, _query: OrderListQuery) -> Vec<Order>, "open orders";
        order_history(&self, _query: OrderListQuery) -> Vec<Order>, "order history";
        fills(&self, _query: FillListQuery) -> Vec<Fill>, "fills";
    }

    async fn ticker_item(&self, symbol: &str) -> Result<Value> {
        let raw = self
            .client
            .ticker(&self.category, symbol)
            .await
            .map_err(Error::from_bybit)?;
        first_list_item(&raw, ExchangeId::Bybit, "Bybit ticker response").cloned()
    }
}

fn first_list_item<'a>(raw: &'a Value, exchange: ExchangeId, context: &str) -> Result<&'a Value> {
    raw.get("list")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("{context} is empty"),
        })
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(|value| match value {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    })
}

fn value_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

enum BybitAccountBillKind {
    All,
    Transfer,
    Deposit,
    Withdrawal,
}

impl BybitAccountBillKind {
    fn from_query(value: Option<&str>) -> Result<Self> {
        match value.map(|value| value.to_ascii_lowercase()) {
            None => Ok(Self::All),
            Some(value) if value == "dnw" || value == "all" => Ok(Self::All),
            Some(value) if value == "transfer" || value == "internal_transfer" => {
                Ok(Self::Transfer)
            }
            Some(value) if value == "deposit" => Ok(Self::Deposit),
            Some(value) if value == "withdraw" || value == "withdrawal" => Ok(Self::Withdrawal),
            Some(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "account bills type filter",
            }),
        }
    }

    fn includes_transfer(&self) -> bool {
        matches!(self, Self::All | Self::Transfer)
    }

    fn includes_deposit(&self) -> bool {
        matches!(self, Self::All | Self::Deposit)
    }

    fn includes_withdrawal(&self) -> bool {
        matches!(self, Self::All | Self::Withdrawal)
    }
}

fn bybit_transfer_request(query: &AccountBillQuery) -> BybitTransferRecordRequest {
    let mut request = BybitTransferRecordRequest::new();
    if let Some(asset) = query.asset.as_deref() {
        request = request.with_coin(asset);
    }
    if let Some(start_time) = query.start_time {
        request = request.with_start_time(start_time);
    }
    if let Some(end_time) = query.end_time {
        request = request.with_end_time(end_time);
    }
    if let Some(limit) = query.limit {
        request = request.with_limit(limit);
    }
    request
}

fn bybit_deposit_request(query: &AccountBillQuery) -> BybitDepositRecordRequest {
    let mut request = BybitDepositRecordRequest::new();
    if let Some(asset) = query.asset.as_deref() {
        request = request.with_coin(asset);
    }
    if let Some(start_time) = query.start_time {
        request = request.with_start_time(start_time);
    }
    if let Some(end_time) = query.end_time {
        request = request.with_end_time(end_time);
    }
    if let Some(limit) = query.limit {
        request = request.with_limit(limit);
    }
    request
}

fn bybit_withdrawal_request(query: &AccountBillQuery) -> BybitWithdrawalRecordRequest {
    let mut request = BybitWithdrawalRecordRequest::new();
    if let Some(asset) = query.asset.as_deref() {
        request = request.with_coin(asset);
    }
    if let Some(start_time) = query.start_time {
        request = request.with_start_time(start_time);
    }
    if let Some(end_time) = query.end_time {
        request = request.with_end_time(end_time);
    }
    if let Some(limit) = query.limit {
        request = request.with_limit(limit);
    }
    request
}

fn bybit_transfer_bills(raw: Value) -> Result<Vec<AccountBill>> {
    let rows = raw
        .get("list")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(rows
        .into_iter()
        .map(|row| bybit_account_bill(row, "transfer", "transferId", &["timestamp", "createdTime"]))
        .collect())
}

fn bybit_deposit_bills(raw: Value) -> Result<Vec<AccountBill>> {
    let rows = raw
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(rows
        .into_iter()
        .map(|row| bybit_account_bill(row, "deposit", "id", &["successAt", "createdTime"]))
        .collect())
}

fn bybit_withdrawal_bills(raw: Value) -> Result<Vec<AccountBill>> {
    let rows = raw
        .get("rows")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(rows
        .into_iter()
        .map(|row| {
            bybit_account_bill(
                row,
                "withdrawal",
                "withdrawID",
                &["updatedTime", "successAt", "createdTime"],
            )
        })
        .collect())
}

fn bybit_account_bill(
    raw: Value,
    bill_type: &str,
    id_field: &str,
    time_fields: &[&str],
) -> AccountBill {
    AccountBill {
        exchange: ExchangeId::Bybit,
        instrument: None,
        exchange_symbol: None,
        bill_id: string_field(&raw, id_field),
        asset: string_field(&raw, "coin"),
        balance_change: string_field(&raw, "amount"),
        balance_after: None,
        fee: string_field(&raw, "fee"),
        pnl: None,
        bill_type: Some(bill_type.to_string()),
        bill_sub_type: string_field(&raw, "status"),
        order_id: None,
        trade_id: None,
        timestamp: first_u64_value(&raw, time_fields),
        raw,
    }
}

fn first_u64_value(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(value_u64))
}

fn bybit_platform_items(raw: Value) -> Vec<Value> {
    raw.get("list")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn bybit_system_status_event(raw: Value) -> PlatformEvent {
    PlatformEvent {
        exchange: ExchangeId::Bybit,
        event_type: "system_status".to_string(),
        event_id: string_field(&raw, "id"),
        title: string_field(&raw, "title").or_else(|| string_field(&raw, "name")),
        status: string_field(&raw, "state").or_else(|| string_field(&raw, "status")),
        url: string_field(&raw, "url"),
        start_time: first_u64_value(&raw, &["begin", "startTime", "startAt"]),
        end_time: first_u64_value(&raw, &["end", "endTime", "endAt"]),
        published_at: first_u64_value(&raw, &["dateTimestamp", "publishTime", "publishedAt"]),
        raw,
    }
}

fn bybit_announcement_event(raw: Value) -> PlatformEvent {
    PlatformEvent {
        exchange: ExchangeId::Bybit,
        event_type: "announcement".to_string(),
        event_id: string_field(&raw, "id").or_else(|| string_field(&raw, "articleId")),
        title: string_field(&raw, "title"),
        status: string_field(&raw, "state").or_else(|| string_field(&raw, "status")),
        url: string_field(&raw, "url"),
        start_time: None,
        end_time: None,
        published_at: first_u64_value(&raw, &["dateTimestamp", "publishTime", "publishedAt"]),
        raw,
    }
}

fn bybit_candle_interval(interval: &str) -> String {
    match interval.trim() {
        "1M" => "M".to_string(),
        value if value.ends_with('m') || value.ends_with('M') => {
            value[..value.len() - 1].to_string()
        }
        value if value.eq_ignore_ascii_case("1h") => "60".to_string(),
        value if value.eq_ignore_ascii_case("2h") => "120".to_string(),
        value if value.eq_ignore_ascii_case("4h") => "240".to_string(),
        value if value.eq_ignore_ascii_case("6h") => "360".to_string(),
        value if value.eq_ignore_ascii_case("12h") => "720".to_string(),
        value if value.eq_ignore_ascii_case("1d") || value.eq_ignore_ascii_case("1Dutc") => {
            "D".to_string()
        }
        value if value.eq_ignore_ascii_case("1w") => "W".to_string(),
        value => value.to_string(),
    }
}

fn bybit_stats_period(period: &str) -> String {
    match period.trim() {
        "5m" => "5min".to_string(),
        "15m" => "15min".to_string(),
        "30m" => "30min".to_string(),
        value => value.to_string(),
    }
}

fn ratio_from_sides(long_ratio: Option<&str>, short_ratio: Option<&str>) -> Option<String> {
    let long = long_ratio?.parse::<f64>().ok()?;
    let short = short_ratio?.parse::<f64>().ok()?;
    if short == 0.0 {
        return None;
    }
    let ratio = long / short;
    Some(
        format!("{ratio:.12}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string(),
    )
}

fn bybit_levels(value: Option<&Value>) -> Vec<OrderBookLevel> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let values = row.as_array()?;
                    Some(OrderBookLevel {
                        price: values.first().and_then(Value::as_str)?.to_string(),
                        size: values.get(1).and_then(Value::as_str)?.to_string(),
                        raw: row.clone(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn bybit_candle(
    exchange: ExchangeId,
    instrument: &Instrument,
    symbol: &str,
    row: Value,
) -> Option<Candle> {
    let values = row.as_array()?;
    Some(Candle {
        exchange,
        instrument: instrument.clone(),
        exchange_symbol: symbol.to_string(),
        open_time: values.first().and_then(value_u64),
        close_time: None,
        open: values.get(1)?.as_str()?.to_string(),
        high: values.get(2)?.as_str()?.to_string(),
        low: values.get(3)?.as_str()?.to_string(),
        close: values.get(4)?.as_str()?.to_string(),
        volume: values.get(5)?.as_str()?.to_string(),
        quote_volume: values.get(6).and_then(Value::as_str).map(ToOwned::to_owned),
        closed: None,
        raw: row,
    })
}

fn order_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    symbol: String,
    raw: Value,
) -> Order {
    Order {
        exchange,
        instrument,
        exchange_symbol: symbol,
        order_id: string_field(&raw, "orderId"),
        client_order_id: string_field(&raw, "orderLinkId"),
        side: string_field(&raw, "side"),
        order_type: string_field(&raw, "orderType"),
        price: string_field(&raw, "price"),
        size: string_field(&raw, "qty"),
        filled_size: string_field(&raw, "cumExecQty"),
        average_price: string_field(&raw, "avgPrice"),
        status: string_field(&raw, "orderStatus"),
        created_at: raw.get("createdTime").and_then(value_u64),
        updated_at: raw.get("updatedTime").and_then(value_u64),
        raw,
    }
}

fn bybit_side(side: OrderSide) -> &'static str {
    match side {
        OrderSide::Buy => "Buy",
        OrderSide::Sell => "Sell",
    }
}

fn bybit_order_type(order_type: OrderType) -> &'static str {
    match order_type {
        OrderType::Market => "Market",
        OrderType::Limit => "Limit",
    }
}

fn bybit_tif(tif: TimeInForce) -> &'static str {
    match tif {
        TimeInForce::Gtc => "GTC",
        TimeInForce::Ioc => "IOC",
        TimeInForce::Fok => "FOK",
        TimeInForce::PostOnly => "PostOnly",
    }
}

fn instrument_from_symbol(symbol: &str) -> Instrument {
    let base = symbol.strip_suffix("USDT").unwrap_or(symbol);
    Instrument::perp(base, "USDT")
}
