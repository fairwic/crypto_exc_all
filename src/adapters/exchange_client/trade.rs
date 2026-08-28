use super::super::*;

impl ExchangeClient {
    pub(crate) async fn place_order(&self, request: PlaceOrderRequest) -> Result<OrderAck> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.place_order(request).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.place_order(request).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.place_order(request).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.place_order(request).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.place_order(request).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "place order",
            }),
        }
    }

    pub(crate) fn trade_capabilities(&self) -> TradeCapabilities {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(_) => TradeCapabilities {
                attached_stop_loss_on_place_order: true,
                attached_take_profit_on_place_order: true,
                protective_order: false,
                protective_stop_amendment: true,
            },
            #[cfg(feature = "binance")]
            Self::Binance(_) => TradeCapabilities {
                attached_stop_loss_on_place_order: false,
                attached_take_profit_on_place_order: false,
                protective_order: true,
                protective_stop_amendment: false,
            },
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => TradeCapabilities {
                attached_stop_loss_on_place_order: true,
                attached_take_profit_on_place_order: false,
                protective_order: false,
                protective_stop_amendment: false,
            },
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => TradeCapabilities {
                attached_stop_loss_on_place_order: false,
                attached_take_profit_on_place_order: false,
                protective_order: false,
                protective_stop_amendment: false,
            },
            #[cfg(feature = "gate")]
            Self::Gate(_) => TradeCapabilities {
                attached_stop_loss_on_place_order: false,
                attached_take_profit_on_place_order: false,
                protective_order: false,
                protective_stop_amendment: false,
            },
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => TradeCapabilities {
                attached_stop_loss_on_place_order: false,
                attached_take_profit_on_place_order: false,
                protective_order: false,
                protective_stop_amendment: false,
            },
        }
    }

    pub(crate) async fn place_protective_order(
        &self,
        request: ProtectiveOrderRequest,
    ) -> Result<OrderAck> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Okx,
                capability: "protective order",
            }),
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.place_protective_order(request).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "protective order",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "protective order",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "protective order",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "protective order",
            }),
        }
    }

    pub(crate) async fn cancel_order(&self, request: CancelOrderRequest) -> Result<OrderAck> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.cancel_order(request).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.cancel_order(request).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.cancel_order(request).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.cancel_order(request).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.cancel_order(request).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "cancel order",
            }),
        }
    }

    pub(crate) async fn cancel_protective_order(
        &self,
        request: CancelOrderRequest,
    ) -> Result<OrderAck> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.cancel_protective_order(request).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.cancel_protective_order(request).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "protective order cancellation",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "protective order cancellation",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "protective order cancellation",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(crate::error::Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "protective order cancellation",
            }),
        }
    }

    pub(crate) async fn amend_protective_stop(
        &self,
        request: AmendProtectiveStopRequest,
    ) -> Result<OrderAck> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.amend_protective_stop(request).await,
            #[cfg(feature = "binance")]
            Self::Binance(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Binance,
                capability: "protective stop amendment",
            }),
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "protective stop amendment",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "protective stop amendment",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "protective stop amendment",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "protective stop amendment",
            }),
        }
    }

    pub(crate) async fn order(&self, query: OrderQuery) -> Result<Order> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.order(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.order(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.order(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.order(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.order(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.order(query).await,
        }
    }

    pub(crate) async fn protective_order(&self, query: ProtectiveOrderQuery) -> Result<Order> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.protective_order(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.protective_order(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.order(query.into_order_query()).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.order(query.into_order_query()).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.order(query.into_order_query()).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "protective order detail",
            }),
        }
    }

    pub(crate) async fn open_protective_orders(&self) -> Result<Vec<Order>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.open_protective_orders().await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.open_protective_orders().await,
            #[cfg(feature = "bitget")]
            Self::Bitget(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bitget,
                capability: "open protective orders",
            }),
            #[cfg(feature = "bybit")]
            Self::Bybit(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Bybit,
                capability: "open protective orders",
            }),
            #[cfg(feature = "gate")]
            Self::Gate(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Gate,
                capability: "open protective orders",
            }),
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(_) => Err(Error::Unsupported {
                exchange: ExchangeId::Hyperliquid,
                capability: "open protective orders",
            }),
        }
    }

    pub(crate) async fn protective_order_history(&self) -> Result<Vec<Order>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.protective_order_history().await,
            _ => Err(Error::Unsupported {
                exchange: self.exchange_id(),
                capability: "protective order history",
            }),
        }
    }

    pub(crate) async fn open_orders(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.open_orders(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.open_orders(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.open_orders(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.open_orders(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.open_orders(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.open_orders(query).await,
        }
    }

    pub(crate) async fn order_history(&self, query: OrderListQuery) -> Result<Vec<Order>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.order_history(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.order_history(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.order_history(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.order_history(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.order_history(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.order_history(query).await,
        }
    }

    pub(crate) async fn fills(&self, query: FillListQuery) -> Result<Vec<Fill>> {
        match self {
            #[cfg(feature = "okx")]
            Self::Okx(adapter) => adapter.fills(query).await,
            #[cfg(feature = "binance")]
            Self::Binance(adapter) => adapter.fills(query).await,
            #[cfg(feature = "bitget")]
            Self::Bitget(adapter) => adapter.fills(query).await,
            #[cfg(feature = "bybit")]
            Self::Bybit(adapter) => adapter.fills(query).await,
            #[cfg(feature = "gate")]
            Self::Gate(adapter) => adapter.fills(query).await,
            #[cfg(feature = "hyperliquid")]
            Self::Hyperliquid(adapter) => adapter.fills(query).await,
        }
    }
}
