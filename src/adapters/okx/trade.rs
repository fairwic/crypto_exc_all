use super::order_mapping::*;
use super::*;

impl OkxAdapter {
    pub(crate) async fn place_order(&self, request: PlaceOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Okx;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let ord_type = order_type(&request);
        let px = if ord_type == "market" {
            request.price.clone()
        } else {
            Some(required_price(exchange, &request)?.to_string())
        };
        let order = OrderReqDto {
            inst_id: symbol.clone(),
            td_mode: okx_margin_mode(request.margin_mode.as_ref()),
            ccy: request.margin_coin.clone(),
            cl_ord_id: request.client_order_id.clone(),
            tag: None,
            side: request.side.lower().to_string(),
            pos_side: request
                .position_side
                .as_deref()
                .map(|value| value.to_ascii_lowercase()),
            ord_type: ord_type.to_string(),
            sz: request.size.clone(),
            px,
            px_usd: None,
            px_vol: None,
            reduce_only: request.reduce_only,
            tgt_ccy: None,
            ban_amend: None,
            quick_mgn_type: None,
            stp_id: None,
            stp_mode: None,
            attach_algo_ords: attached_exit_orders(&request),
        };
        let mut response = self
            .trade
            .place_order(order)
            .await
            .map_err(Error::from_okx_mutation)?;
        let order = response.drain(..).next().ok_or_else(|| Error::Adapter {
            exchange,
            message: "OKX order response is empty".to_string(),
        })?;
        okx_order_ack_from_response(instrument, symbol, order)
    }

    pub(crate) async fn cancel_order(&self, request: CancelOrderRequest) -> Result<OrderAck> {
        let exchange = ExchangeId::Okx;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        if request.order_id.is_none() && request.client_order_id.is_none() {
            return Err(missing_cancel_id(exchange));
        }

        let raw = self
            .trade
            .cancel_order(
                &symbol,
                request.order_id.as_deref(),
                request.client_order_id.as_deref(),
            )
            .await
            .map_err(Error::from_okx_mutation)?;
        let item = first_object_value(raw, exchange, "OKX cancel response")?;
        let object = item.as_object().ok_or_else(|| Error::Adapter {
            exchange,
            message: "OKX cancel response item is not an object".to_string(),
        })?;

        Ok(OrderAck {
            exchange,
            instrument,
            exchange_symbol: symbol,
            order_id: string_field(object, "ordId"),
            client_order_id: string_field(object, "clOrdId"),
            status: string_field(object, "sCode"),
            raw: item,
        })
    }

    /// 撤销已由主订单附带创建的 OKX 止损算法订单。
    pub(crate) async fn cancel_protective_order(
        &self,
        request: CancelOrderRequest,
    ) -> Result<OrderAck> {
        let exchange = ExchangeId::Okx;
        let instrument = request.instrument.clone();
        let symbol = instrument.symbol_for(exchange);
        let algo_id = request.order_id.as_deref().ok_or_else(|| Error::Adapter {
            exchange,
            message: "OKX protective cancellation requires algo order id".to_owned(),
        })?;
        let raw = self
            .trade
            .cancel_algo_order(&symbol, algo_id)
            .await
            .map_err(Error::from_okx_mutation)?;
        let item = first_object_value(raw, exchange, "OKX cancel algo response")?;
        let object = item.as_object().ok_or_else(|| Error::Adapter {
            exchange,
            message: "OKX cancel algo response item is not an object".to_owned(),
        })?;
        let status = string_field(object, "sCode").unwrap_or_default();
        if status != "0" {
            return Err(Error::Api {
                exchange,
                status: Some(200),
                code: status,
                message: string_field(object, "sMsg")
                    .unwrap_or_else(|| "OKX cancel algo order rejected".to_owned()),
            });
        }

        Ok(OrderAck {
            exchange,
            instrument,
            exchange_symbol: symbol,
            order_id: string_field(object, "algoId").or_else(|| Some(algo_id.to_owned())),
            client_order_id: request.client_order_id,
            status: Some(status),
            raw: item,
        })
    }

    pub(crate) async fn order(&self, query: OrderQuery) -> Result<Order> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        if query.order_id.is_none() && query.client_order_id.is_none() {
            return Err(missing_order_query_id(exchange));
        }

        let mut response = self
            .trade
            .get_order_details(
                &symbol,
                query.order_id.as_deref(),
                query.client_order_id.as_deref(),
            )
            .await
            .map_err(Error::from_okx)?;
        let order = response.drain(..).next().ok_or_else(|| Error::Adapter {
            exchange,
            message: format!("OKX order response is empty for {symbol}"),
        })?;

        okx_order_from_detail(exchange, Some(instrument), Some(symbol), order)
    }

    /// 查询由主订单附带创建的 OKX 止损算法订单，不把它误当普通订单查询。
    pub(crate) async fn protective_order(&self, query: ProtectiveOrderQuery) -> Result<Order> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument.symbol_for(exchange);
        if query.order_id.is_none() && query.client_order_id.is_none() {
            return Err(missing_order_query_id(exchange));
        }
        let raw = self
            .trade
            .get_algo_order_details(query.order_id.as_deref(), query.client_order_id.as_deref())
            .await
            .map_err(Error::from_okx)?;
        let item = first_object_value(raw, exchange, "OKX protective order response")?;
        okx_algo_order_from_value(exchange, instrument, symbol, item)
    }

    pub(crate) async fn open_orders(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument
            .as_ref()
            .map(|instrument| instrument.symbol_for(exchange));
        let response = self
            .trade
            .get_pending_orders(
                Some("SWAP"),
                symbol.as_deref(),
                None,
                query.status.as_deref(),
                query.after.as_deref(),
                query.before.as_deref(),
                query.limit,
            )
            .await
            .map_err(Error::from_okx)?;

        response
            .into_iter()
            .map(|order| {
                okx_order_from_pending(exchange, instrument.clone(), symbol.clone(), order)
            })
            .collect()
    }

    pub(crate) async fn order_history(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument
            .as_ref()
            .map(|instrument| instrument.symbol_for(exchange));
        let use_archive = query.start_time.is_some() || query.end_time.is_some();
        let request = OrdListReqDto {
            inst_type: "SWAP".to_string(),
            inst_id: symbol.clone(),
            ord_type: None,
            state: query.status,
            after: query.after,
            before: query.before,
            begin: query.start_time.map(|value| value.to_string()),
            end: query.end_time.map(|value| value.to_string()),
            limit: query.limit,
        };
        let response = if use_archive {
            self.trade.get_order_history_archive(request).await
        } else {
            self.trade.get_order_history(request).await
        }
        .map_err(Error::from_okx)?;

        response
            .into_iter()
            .map(|order| okx_order_from_detail(exchange, instrument.clone(), symbol.clone(), order))
            .collect()
    }

    pub(crate) async fn fills(&self, query: FillListQuery) -> Result<Vec<Fill>> {
        let exchange = ExchangeId::Okx;
        let instrument = query.instrument;
        let symbol = instrument
            .as_ref()
            .map(|instrument| instrument.symbol_for(exchange));
        let begin = query.start_time.map(|value| value.to_string());
        let end = query.end_time.map(|value| value.to_string());
        let use_history = begin.is_some() || end.is_some();
        let raw = if use_history {
            self.trade
                .get_fills_history(
                    Some("SWAP"),
                    symbol.as_deref(),
                    query.order_id.as_deref(),
                    query.after.as_deref(),
                    query.before.as_deref(),
                    begin.as_deref(),
                    end.as_deref(),
                    query.limit,
                )
                .await
        } else {
            self.trade
                .get_fills(
                    Some("SWAP"),
                    symbol.as_deref(),
                    query.order_id.as_deref(),
                    query.after.as_deref(),
                    query.before.as_deref(),
                    query.limit,
                )
                .await
        }
        .map_err(Error::from_okx)?;

        okx_fills_from_value(exchange, instrument, symbol, raw, "OKX fills response")
    }
}
