use super::*;

pub(super) fn okx_order_ack_from_response(
    instrument: Instrument,
    symbol: String,
    order: OrderResDto,
) -> Result<OrderAck> {
    let exchange = ExchangeId::Okx;
    if order.s_code.trim() != "0" {
        return Err(Error::Api {
            exchange,
            status: Some(200),
            code: order.s_code,
            message: order
                .s_msg
                .unwrap_or_else(|| "OKX order rejected".to_string()),
        });
    }
    let raw = serde_json::to_value(&order)?;

    Ok(OrderAck {
        exchange,
        instrument,
        exchange_symbol: symbol,
        order_id: non_empty(order.ord_id),
        client_order_id: order.cl_ord_id.and_then(non_empty),
        status: Some(order.s_code),
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

fn okx_order_type_from_enum(value: OkxRawOrderType) -> &'static str {
    match value {
        OkxRawOrderType::Market => "market",
        OkxRawOrderType::Limit => "limit",
        OkxRawOrderType::PostOnly => "post_only",
        OkxRawOrderType::FillOrKill => "fok",
        OkxRawOrderType::ImmediateOrCancel => "ioc",
        OkxRawOrderType::OptimalLimitIoc => "optimal_limit_ioc",
    }
}

pub(super) fn okx_order_from_detail(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    order: OrderDetailRespDto,
) -> Result<Order> {
    let raw = serde_json::to_value(&order)?;
    let exchange_symbol = if order.inst_id.is_empty() {
        symbol_hint.unwrap_or_default()
    } else {
        order.inst_id.clone()
    };
    let mapped_instrument =
        instrument.unwrap_or_else(|| instrument_from_okx_symbol(&exchange_symbol));

    Ok(Order {
        exchange,
        instrument: mapped_instrument,
        exchange_symbol,
        order_id: non_empty(order.ord_id),
        client_order_id: non_empty(order.cl_ord_id),
        side: non_empty(order.side),
        order_type: non_empty(order.ord_type),
        price: non_empty(order.px),
        size: non_empty(order.sz),
        filled_size: non_empty(order.acc_fill_sz),
        average_price: non_empty(order.avg_px),
        status: non_empty(order.state),
        created_at: parse_u64_string(&order.c_time),
        updated_at: parse_u64_string(&order.u_time),
        raw,
    })
}

pub(super) fn okx_order_from_pending(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    order: OrderPendingRespDto,
) -> Result<Order> {
    let raw = serde_json::to_value(&order)?;
    let exchange_symbol = if order.inst_id.is_empty() {
        symbol_hint.unwrap_or_default()
    } else {
        order.inst_id.clone()
    };
    let mapped_instrument =
        instrument.unwrap_or_else(|| instrument_from_okx_symbol(&exchange_symbol));

    Ok(Order {
        exchange,
        instrument: mapped_instrument,
        exchange_symbol,
        order_id: non_empty(order.order_id),
        client_order_id: order.client_order_id.and_then(non_empty),
        side: Some(order.side.as_str().to_string()),
        order_type: Some(okx_order_type_from_enum(order.order_type).to_string()),
        price: non_empty(order.px),
        size: non_empty(order.sz),
        filled_size: order.filled_size.and_then(non_empty),
        average_price: order.filled_price.and_then(non_empty),
        status: non_empty(order.state),
        created_at: parse_u64_string(&order.creation_time),
        updated_at: order.update_time.as_deref().and_then(parse_u64_string),
        raw,
    })
}

/// 把 OKX `order-algo` 的条件止损详情映射到统一只读 Order contract。
pub(super) fn okx_algo_order_from_value(
    exchange: ExchangeId,
    instrument: Option<Instrument>,
    symbol_hint: Option<String>,
    raw: Value,
) -> Result<Order> {
    let object = raw.as_object().ok_or_else(|| Error::Adapter {
        exchange,
        message: "OKX protective order response item is not an object".to_string(),
    })?;
    let exchange_symbol = string_field(object, "instId")
        .or(symbol_hint)
        .ok_or_else(|| Error::Adapter {
            exchange,
            message: "OKX protective order response is missing instId".to_string(),
        })?;
    let instrument = instrument.unwrap_or_else(|| instrument_from_okx_symbol(&exchange_symbol));
    Ok(Order {
        exchange,
        instrument,
        exchange_symbol,
        order_id: string_field(object, "algoId"),
        client_order_id: string_field(object, "algoClOrdId"),
        side: string_field(object, "side"),
        order_type: string_field(object, "ordType"),
        price: first_string_field(object, &["slTriggerPx", "triggerPx", "tpTriggerPx"]),
        size: string_field(object, "sz"),
        filled_size: string_field(object, "actualSz"),
        average_price: string_field(object, "actualPx"),
        status: string_field(object, "state"),
        created_at: u64_field(object, "cTime"),
        updated_at: u64_field(object, "uTime"),
        raw,
    })
}
