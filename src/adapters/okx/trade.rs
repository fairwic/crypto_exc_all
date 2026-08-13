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

    /// 原位修改已由主订单附带创建的 OKX 止损算法订单。
    pub(crate) async fn amend_protective_stop(
        &self,
        request: AmendProtectiveStopRequest,
    ) -> Result<OrderAck> {
        let exchange = ExchangeId::Okx;
        let instrument = request.instrument;
        let symbol = instrument.symbol_for(exchange);
        if request.order_id.is_empty()
            || request.quantity.is_empty()
            || request.stop_price.is_empty()
            || !(1..=32).contains(&request.request_id.len())
            || !request
                .request_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric())
        {
            return Err(Error::Adapter {
                exchange,
                message: "OKX protective stop amendment requires algo id, quantity, stop price, and 1-32 character alphanumeric request id".to_owned(),
            });
        }
        let trigger_price_type = match request.working_type {
            ProtectiveOrderWorkingType::MarkPrice => "mark",
            ProtectiveOrderWorkingType::ContractPrice => "last",
        };
        let take_profit_price = request.take_profit_price;
        let raw = self
            .trade
            .amend_algo_order(AmendAlgoOrderReqDto {
                inst_id: symbol.clone(),
                algo_id: request.order_id.clone(),
                cxl_on_fail: false,
                req_id: request.request_id,
                new_sz: request.quantity,
                new_tp_trigger_px: take_profit_price.clone(),
                new_tp_ord_px: take_profit_price.as_ref().map(|_| "-1".to_owned()),
                new_tp_trigger_px_type: take_profit_price
                    .as_ref()
                    .map(|_| trigger_price_type.to_owned()),
                new_sl_trigger_px: request.stop_price,
                new_sl_ord_px: "-1".to_owned(),
                new_sl_trigger_px_type: trigger_price_type.to_owned(),
            })
            .await
            .map_err(Error::from_okx_mutation)?;
        let item = first_object_value(raw, exchange, "OKX amend algo response")?;
        let object = item.as_object().ok_or_else(|| Error::Adapter {
            exchange,
            message: "OKX amend algo response item is not an object".to_owned(),
        })?;
        let status = string_field(object, "sCode").unwrap_or_default();
        if status != "0" {
            return Err(Error::Api {
                exchange,
                status: Some(200),
                code: status,
                message: string_field(object, "sMsg")
                    .unwrap_or_else(|| "OKX amend algo order rejected".to_owned()),
            });
        }

        Ok(OrderAck {
            exchange,
            instrument,
            exchange_symbol: symbol,
            order_id: string_field(object, "algoId").or(Some(request.order_id)),
            client_order_id: string_field(object, "reqId"),
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
        okx_algo_order_from_value(exchange, Some(instrument), Some(symbol), item)
    }

    /// 读取账户级全部未触发 SWAP 条件单；满页时拒绝把不完整结果当作安全证据。
    pub(crate) async fn open_protective_orders(&self) -> Result<Vec<Order>> {
        let exchange = ExchangeId::Okx;
        let response = self
            .trade
            .get_pending_algo_orders("SWAP", "conditional", 100)
            .await
            .map_err(Error::from_okx)?;
        if response.len() >= 100 {
            return Err(Error::Adapter {
                exchange,
                message: "OKX pending protective order result reached page limit".to_owned(),
            });
        }
        response
            .into_iter()
            .map(|order| okx_algo_order_from_value(exchange, None, None, order))
            .collect()
    }

    /// 读取条件保护单的封闭历史状态；每个状态满页时拒绝返回不完整证据。
    pub(crate) async fn protective_order_history(&self) -> Result<Vec<Order>> {
        let exchange = ExchangeId::Okx;
        let mut orders = Vec::new();
        for state in ["effective", "canceled", "order_failed"] {
            let response = self
                .trade
                .get_algo_order_history("SWAP", "conditional", state, 100)
                .await
                .map_err(Error::from_okx)?;
            if response.len() >= 100 {
                return Err(Error::Adapter {
                    exchange,
                    message: format!(
                        "OKX historical protective order result reached page limit for {state}"
                    ),
                });
            }
            orders.extend(
                response
                    .into_iter()
                    .map(|order| okx_algo_order_from_value(exchange, None, None, order))
                    .collect::<Result<Vec<_>>>()?,
            );
        }
        Ok(orders)
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
