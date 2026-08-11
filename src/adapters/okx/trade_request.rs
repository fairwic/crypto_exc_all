use crate::trade::{OrderType, PlaceOrderRequest, TimeInForce};
use okx_rs::dto::trade_dto::AttachAlgoOrdReqDto;

/// 将统一订单类型与时效约束映射成 OKX `ordType`。
pub(super) fn order_type(request: &PlaceOrderRequest) -> &'static str {
    match (request.order_type, request.time_in_force) {
        (_, Some(TimeInForce::PostOnly)) => "post_only",
        (_, Some(TimeInForce::Ioc)) => "ioc",
        (_, Some(TimeInForce::Fok)) => "fok",
        (OrderType::Limit, _) => "limit",
        (OrderType::Market, _) => "market",
    }
}

/// 将随主单提交的单目标止盈与强制止损映射为同一个 OKX 附带算法订单。
pub(super) fn attached_exit_orders(
    request: &PlaceOrderRequest,
) -> Option<Vec<AttachAlgoOrdReqDto>> {
    let stop_loss_price = request
        .attached_stop_loss_price
        .as_ref()
        .filter(|price| !price.trim().is_empty());
    let take_profit_price = request
        .attached_take_profit_price
        .as_ref()
        .filter(|price| !price.trim().is_empty());
    if stop_loss_price.is_none() && take_profit_price.is_none() {
        return None;
    }

    Some(vec![AttachAlgoOrdReqDto {
        attach_algo_cl_ord_id: request.attached_stop_loss_client_order_id.clone(),
        tp_trigger_px: take_profit_price.cloned(),
        tp_ord_px: take_profit_price.map(|_| "-1".to_string()),
        tp_ord_kind: None,
        tp_trigger_px_type: take_profit_price.map(|_| "mark".to_string()),
        sl_trigger_px: stop_loss_price.cloned(),
        sl_ord_px: stop_loss_price.map(|_| "-1".to_string()),
        sl_trigger_px_type: stop_loss_price.map(|_| "mark".to_string()),
        sz: None,
        amend_px_on_trigger_type: None,
    }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instrument::Instrument;
    use crate::trade::OrderSide;

    #[test]
    fn attached_exit_prices_map_to_one_market_algo_order() {
        let request =
            PlaceOrderRequest::market(Instrument::perp("ETH", "USDT"), OrderSide::Buy, "0.1")
                .with_attached_stop_loss_price("2200.5")
                .with_attached_take_profit_price("2800");

        let attached = attached_exit_orders(&request).expect("attached exit mapping");
        assert_eq!(attached.len(), 1);
        assert_eq!(attached[0].sl_trigger_px.as_deref(), Some("2200.5"));
        assert_eq!(attached[0].sl_ord_px.as_deref(), Some("-1"));
        assert_eq!(attached[0].sl_trigger_px_type.as_deref(), Some("mark"));
        assert_eq!(attached[0].tp_trigger_px.as_deref(), Some("2800"));
        assert_eq!(attached[0].tp_ord_px.as_deref(), Some("-1"));
        assert_eq!(attached[0].tp_trigger_px_type.as_deref(), Some("mark"));
    }
}
