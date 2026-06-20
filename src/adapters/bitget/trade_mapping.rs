use super::*;

pub(super) fn bitget_apply_attached_stop_loss(
    request: BitgetNewOrderRequest,
    source: &PlaceOrderRequest,
) -> BitgetNewOrderRequest {
    match source
        .attached_stop_loss_price
        .as_ref()
        .filter(|price| !price.trim().is_empty())
    {
        Some(price) => request.with_preset_stop_loss_price(price.clone()),
        None => request,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade::OrderSide;

    #[test]
    fn bitget_attached_stop_loss_maps_to_preset_stop_loss_price() {
        let source =
            PlaceOrderRequest::market(Instrument::perp("ETH", "USDT"), OrderSide::Buy, "0.1")
                .with_attached_stop_loss_price("2200.5");
        let target = BitgetNewOrderRequest::market(
            "ETHUSDT",
            "USDT-FUTURES",
            "isolated",
            "USDT",
            "0.1",
            "buy",
        );

        let mapped = bitget_apply_attached_stop_loss(target, &source);

        assert_eq!(mapped.preset_stop_loss_price.as_deref(), Some("2200.5"));
    }
}

pub(super) fn order_ack_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    exchange_symbol: String,
    raw: Value,
    label: &str,
) -> Result<OrderAck> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: format!("{label} is not an object"),
    })?;

    Ok(OrderAck {
        exchange,
        instrument,
        exchange_symbol,
        order_id: string_field(object, "orderId"),
        client_order_id: string_field(object, "clientOid"),
        status: first_string_field(object, &["status", "state"]),
        raw,
    })
}

pub(super) fn missing_cancel_id(exchange: ExchangeId) -> Error {
    Error::Adapter {
        exchange,
        message: "cancel_order requires order_id or client_order_id".to_string(),
    }
}

pub(super) fn missing_order_query_id(exchange: ExchangeId) -> Error {
    Error::Adapter {
        exchange,
        message: "order query requires order_id or client_order_id".to_string(),
    }
}

pub(super) fn bitget_order_query_request(
    product_type: &str,
    query: &OrderListQuery,
    symbol: Option<&str>,
) -> BitgetOrderQueryRequest {
    let mut request = BitgetOrderQueryRequest::new(product_type);
    if let Some(symbol) = symbol {
        request = request.with_symbol(symbol);
    }
    if let Some(status) = query.status.as_deref() {
        request = request.with_status(status);
    }
    if let Some(before) = query.before.as_deref() {
        request = request.with_id_less_than(before);
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

pub(super) fn bitget_fill_query_request(
    product_type: &str,
    query: &FillListQuery,
    symbol: Option<&str>,
) -> BitgetOrderQueryRequest {
    let mut request = BitgetOrderQueryRequest::new(product_type);
    if let Some(symbol) = symbol {
        request = request.with_symbol(symbol);
    }
    if let Some(order_id) = query.order_id.as_deref() {
        request = request.with_order_id(order_id);
    }
    if let Some(before) = query.before.as_deref() {
        request = request.with_id_less_than(before);
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

pub(super) fn bitget_orders_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    raw: Value,
    label: &str,
) -> Result<Vec<Order>> {
    owned_order_items(raw, exchange, label)?
        .into_iter()
        .map(|value| {
            bitget_order_from_value(
                exchange,
                instrument.clone(),
                symbol_hint.clone(),
                value,
                "Bitget order item",
            )
        })
        .collect()
}

pub(super) fn bitget_order_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    raw: Value,
    label: &str,
) -> Result<Order> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: format!("{label} is not an object"),
    })?;
    let exchange_symbol = first_string_field(object, &["symbol", "instId"])
        .or(symbol_hint)
        .unwrap_or_default();
    let mapped_instrument =
        instrument.unwrap_or_else(|| instrument_from_linear_symbol(&exchange_symbol));

    Ok(Order {
        exchange,
        instrument: mapped_instrument,
        exchange_symbol,
        order_id: first_string_field(object, &["orderId", "ordId"]),
        client_order_id: first_string_field(object, &["clientOid", "clientOrderId", "clOrdId"]),
        side: string_field(object, "side"),
        order_type: first_string_field(object, &["orderType", "ordType"]),
        price: string_field(object, "price"),
        size: first_string_field(object, &["size", "sz", "origQty"]),
        filled_size: first_string_field(
            object,
            &["baseVolume", "filledQty", "filledSize", "fillSz"],
        ),
        average_price: first_string_field(object, &["priceAvg", "avgPrice", "averagePrice"]),
        status: first_string_field(object, &["status", "state"]),
        created_at: u64_field(object, "cTime").or_else(|| u64_field(object, "time")),
        updated_at: u64_field(object, "uTime").or_else(|| u64_field(object, "updateTime")),
        raw,
    })
}

pub(super) fn bitget_orderbook_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    exchange_symbol: String,
    raw: Value,
) -> Result<OrderBook> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "Bitget orderbook response is not an object".to_string(),
    })?;

    Ok(OrderBook {
        exchange,
        instrument,
        exchange_symbol,
        bids: bitget_book_levels(object.get("bids"), exchange, "bids")?,
        asks: bitget_book_levels(object.get("asks"), exchange, "asks")?,
        timestamp: u64_field(object, "ts").or_else(|| u64_field(object, "time")),
        raw,
    })
}

pub(super) fn bitget_book_levels(
    value: Option<&Value>,
    exchange: ExchangeId,
    side: &str,
) -> Result<Vec<OrderBookLevel>> {
    let Some(Value::Array(levels)) = value else {
        return Err(Error::Adapter {
            exchange,
            message: format!("Bitget orderbook {side} is not an array"),
        });
    };

    levels
        .iter()
        .map(|level| {
            let values = level.as_array().ok_or_else(|| Error::Adapter {
                exchange,
                message: format!("Bitget orderbook {side} level is not an array"),
            })?;
            Ok(OrderBookLevel {
                price: value_string_at(values, 0).unwrap_or_default(),
                size: value_string_at(values, 1).unwrap_or_default(),
                raw: level.clone(),
            })
        })
        .collect()
}

pub(super) fn bitget_candles_from_value(
    exchange: ExchangeId,
    instrument: Instrument,
    exchange_symbol: String,
    raw: Value,
) -> Result<Vec<Candle>> {
    let Value::Array(items) = raw else {
        return Err(Error::Adapter {
            exchange,
            message: "Bitget candles response is not an array".to_string(),
        });
    };

    items
        .into_iter()
        .map(|item| {
            let values = item.as_array().ok_or_else(|| Error::Adapter {
                exchange,
                message: "Bitget candle item is not an array".to_string(),
            })?;
            Ok(Candle {
                exchange,
                instrument: instrument.clone(),
                exchange_symbol: exchange_symbol.clone(),
                open_time: value_u64_at(values, 0),
                close_time: None,
                open: value_string_at(values, 1).unwrap_or_default(),
                high: value_string_at(values, 2).unwrap_or_default(),
                low: value_string_at(values, 3).unwrap_or_default(),
                close: value_string_at(values, 4).unwrap_or_default(),
                volume: value_string_at(values, 5).unwrap_or_default(),
                quote_volume: value_string_at(values, 6),
                closed: None,
                raw: item,
            })
        })
        .collect()
}

pub(super) fn bitget_fills_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    raw: Value,
    label: &str,
) -> Result<Vec<Fill>> {
    owned_fill_items(raw, exchange, label)?
        .into_iter()
        .map(|value| {
            let object = value.as_object().ok_or_else(|| Error::Adapter {
                exchange,
                message: "Bitget fill item is not an object".to_string(),
            })?;
            let exchange_symbol = first_string_field(object, &["symbol", "instId"])
                .or_else(|| symbol_hint.clone())
                .unwrap_or_default();
            let mapped_instrument = instrument
                .clone()
                .unwrap_or_else(|| instrument_from_linear_symbol(&exchange_symbol));

            Ok(Fill {
                exchange,
                instrument: mapped_instrument,
                exchange_symbol,
                trade_id: first_string_field(object, &["tradeId", "fillId", "id"]),
                order_id: first_string_field(object, &["orderId", "ordId"]),
                side: string_field(object, "side"),
                price: string_field(object, "price"),
                size: first_string_field(object, &["baseVolume", "fillSz", "size", "qty"]),
                fee: string_field(object, "fee"),
                fee_asset: first_string_field(object, &["feeCcy", "feeCoin", "feeAsset"]),
                role: first_string_field(object, &["role", "execType"]),
                timestamp: u64_field(object, "cTime")
                    .or_else(|| u64_field(object, "ts"))
                    .or_else(|| u64_field(object, "time")),
                raw: value,
            })
        })
        .collect()
}
