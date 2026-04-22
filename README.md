# crypto_exc_all

统一加密货币交易所 SDK facade。外部业务只依赖 `crypto_exc_all`，由根 crate 自动加载不同交易所的 API key，并在内部转成对应的 `okx_rs` / `binance_rs` / `bitget_rs` client。

## 发布结构

本项目按三层发布：

- `okx_rs`: OKX 交易所 SDK crate。
- `binance_rs`: Binance USDⓈ-M Futures SDK crate。
- `bitget_rs`: Bitget V2 Futures SDK crate。
- `crypto_exc_all`: 统一入口 crate，通过版本依赖引用各交易所 SDK。

本地开发使用 `path + version` 依赖；发布时先发布子交易所 crate，再发布 `crypto_exc_all`。

```toml
[dependencies]
crypto_exc_all = "0.1"
```

当前根 crate 的默认 feature 会启用 OKX、Binance 和 Bitget：

```toml
crypto_exc_all = { version = "0.1", default-features = true }
```

只启用 Binance：

```toml
crypto_exc_all = { version = "0.1", default-features = false, features = ["binance"] }
```

只启用 Bitget：

```toml
crypto_exc_all = { version = "0.1", default-features = false, features = ["bitget"] }
```

## 环境变量

OKX:

```env
OKX_API_KEY=...
OKX_API_SECRET=...
OKX_PASSPHRASE=...
OKX_SIMULATED_TRADING=1
```

也支持 OKX 模拟盘变量：

```env
OKX_SIMULATED_API_KEY=...
OKX_SIMULATED_API_SECRET=...
OKX_SIMULATED_PASSPHRASE=...
```

Binance:

```env
BINANCE_API_KEY=...
BINANCE_API_SECRET=...
BINANCE_PROXY_URL=socks5h://127.0.0.1:7897
```

`BINANCE_PROXY_URL` 可省略；如果传入 `socks5://`，SDK 会自动规范化成 `socks5h://`。

Bitget:

```env
BITGET_API_KEY=...
BITGET_API_SECRET=...
BITGET_PASSPHRASE=...
BITGET_PRODUCT_TYPE=USDT-FUTURES
BITGET_PROXY_URL=socks5h://127.0.0.1:7897
BITGET_WS_PUBLIC_URL=wss://ws.bitget.com/v2/ws/public
BITGET_WS_PRIVATE_URL=wss://ws.bitget.com/v2/ws/private
```

`BITGET_PRODUCT_TYPE` 可省略，根 adapter 默认使用 `USDT-FUTURES`。也支持 `.env` 中的小写/既有混合大小写变量名：`bitget_api_key`、`bitget_api_secret`、`bitget_passphrase`、`bitget_PASSPHRASE`。
`BITGET_WS_PUBLIC_URL` / `BITGET_WS_PRIVATE_URL` 可省略，`bitget_rs` 会默认使用 Bitget V2 public/private WebSocket 主域名。

## 统一调用

```rust
use crypto_exc_all::{CryptoSdk, ExchangeId, Instrument};

#[tokio::main]
async fn main() -> crypto_exc_all::Result<()> {
    let sdk = CryptoSdk::from_env()?;
    let instrument = Instrument::perp("BTC", "USDT");

    let ticker = sdk
        .market(ExchangeId::Bitget)?
        .ticker(&instrument)
        .await?;

    println!("{} {}", ticker.exchange_symbol, ticker.last_price);

    Ok(())
}
```

遍历所有已配置交易所：

```rust
use crypto_exc_all::{CryptoSdk, Instrument};

#[tokio::main]
async fn main() -> crypto_exc_all::Result<()> {
    let sdk = CryptoSdk::from_env()?;
    let instrument = Instrument::perp("BTC", "USDT");

    for exchange in sdk.configured_exchanges() {
        let ticker = sdk.market(exchange)?.ticker(&instrument).await?;
        println!("{exchange}: {} {}", ticker.exchange_symbol, ticker.last_price);
    }

    Ok(())
}
```

运行示例：

```bash
cargo run --example unified_market
```

统一持仓、交易和订单查询入口：

```rust
use crypto_exc_all::{
    CancelOrderRequest, CryptoSdk, EnsureOrderMarginModeRequest, ExchangeId, Instrument,
    MarginMode, OrderSide, PlaceOrderRequest, PositionMode,
    CandleQuery, FillListQuery, FundingRateQuery, MarketStatsQuery, OrderBookQuery,
    OrderListQuery, PrepareOrderSettingsRequest, SetLeverageRequest, SetPositionModeRequest,
    TimeInForce,
};

#[tokio::main]
async fn main() -> crypto_exc_all::Result<()> {
    let sdk = CryptoSdk::from_env()?;
    let instrument = Instrument::perp("BTC", "USDT");

    let positions = sdk
        .positions(ExchangeId::Bitget)?
        .list(Some(&instrument))
        .await?;
    println!("positions={positions:?}");

    let book = sdk
        .market(ExchangeId::Bitget)?
        .orderbook(OrderBookQuery::new(instrument.clone()).with_limit(20))
        .await?;
    println!("best_bid={:?} best_ask={:?}", book.bids.first(), book.asks.first());

    let candles = sdk
        .market(ExchangeId::Bitget)?
        .candles(CandleQuery::new(instrument.clone(), "1m").with_limit(100))
        .await?;
    println!("candles={candles:?}");

    let funding = sdk
        .market(ExchangeId::Bitget)?
        .funding_rate(&instrument)
        .await?;
    println!("funding={funding:?}");

    let funding_history = sdk
        .market(ExchangeId::Bitget)?
        .funding_rate_history(FundingRateQuery::new(instrument.clone()).with_limit(20))
        .await?;
    println!("funding_history={funding_history:?}");

    let mark_price = sdk
        .market(ExchangeId::Bitget)?
        .mark_price(&instrument)
        .await?;
    println!("mark_price={mark_price:?}");

    let open_interest = sdk
        .market(ExchangeId::Bitget)?
        .open_interest(&instrument)
        .await?;
    println!("open_interest={open_interest:?}");

    let sentiment_query = MarketStatsQuery::new(instrument.clone(), "5m").with_limit(20);
    let long_short = sdk
        .market(ExchangeId::Bitget)?
        .long_short_ratio(sentiment_query.clone())
        .await?;
    println!("long_short={long_short:?}");

    let taker_volume = sdk
        .market(ExchangeId::Bitget)?
        .taker_buy_sell_volume(sentiment_query)
        .await?;
    println!("taker_volume={taker_volume:?}");

    let open_orders = sdk
        .orders(ExchangeId::Bitget)?
        .open(OrderListQuery::for_instrument(instrument.clone()).with_limit(20))
        .await?;
    println!("open_orders={open_orders:?}");

    let fills = sdk
        .fills(ExchangeId::Bitget)?
        .list(FillListQuery::for_instrument(instrument.clone()).with_limit(20))
        .await?;
    println!("fills={fills:?}");

    let leverage = sdk
        .account(ExchangeId::Bitget)?
        .set_leverage(
            SetLeverageRequest::new(instrument.clone(), "20")
                .with_margin_mode(MarginMode::Cross)
                .with_margin_coin("USDT"),
        )
        .await?;
    println!("leverage={leverage:?}");

    let order_margin_mode = sdk
        .account(ExchangeId::Bitget)?
        .ensure_order_margin_mode(
            EnsureOrderMarginModeRequest::new(instrument.clone(), MarginMode::Cross)
                .with_product_type("USDT-FUTURES")
                .with_margin_coin("USDT"),
        )
        .await?;
    println!("order_margin_mode={order_margin_mode:?}");

    let order_settings = sdk
        .account(ExchangeId::Bitget)?
        .prepare_order_settings(
            PrepareOrderSettingsRequest::new(instrument.clone())
                .with_position_mode(PositionMode::Hedge)
                .with_margin_mode(MarginMode::Cross)
                .with_leverage("20")
                .with_product_type("USDT-FUTURES")
                .with_margin_coin("USDT")
                .with_position_side("long"),
        )
        .await?;
    println!("order_settings={order_settings:?}");

    let position_mode = sdk
        .account(ExchangeId::Bitget)?
        .set_position_mode(
            SetPositionModeRequest::new(PositionMode::Hedge)
                .with_product_type("USDT-FUTURES"),
        )
        .await?;
    println!("position_mode={position_mode:?}");

    let order = sdk
        .trade(ExchangeId::Bitget)?
        .place_order(
            PlaceOrderRequest::limit(instrument.clone(), OrderSide::Buy, "0.001", "60000")
                .with_time_in_force(TimeInForce::PostOnly)
                .with_client_order_id("my-client-order-id"),
        )
        .await?;

    if let Some(order_id) = order.order_id {
        sdk.trade(ExchangeId::Bitget)?
            .cancel_order(CancelOrderRequest::by_order_id(instrument, order_id))
            .await?;
    }

    Ok(())
}
```

## 当前统一能力

- 自动读取 OKX / Binance / Bitget 凭证。
- `CryptoSdk::from_env()` / `CryptoSdk::from_config()`。
- `sdk.configured_exchanges()`。
- 统一 `Instrument`，自动映射交易所 symbol：
  - Binance 永续：`BTCUSDT`
  - OKX 永续：`BTC-USDT-SWAP`
  - Bitget USDT 永续：`BTCUSDT`
- 统一 market ticker：
  - `sdk.market(exchange)?.ticker(&instrument).await`
- 统一 market orderbook 和 candles：
  - `sdk.market(exchange)?.orderbook(query).await`
  - `sdk.market(exchange)?.candles(query).await`
- 统一 derivatives market metrics：
  - `sdk.market(exchange)?.funding_rate(&instrument).await`
  - `sdk.market(exchange)?.funding_rate_history(query).await`
  - `sdk.market(exchange)?.mark_price(&instrument).await`
  - `sdk.market(exchange)?.open_interest(&instrument).await`
- 统一 market sentiment stats：
  - `sdk.market(exchange)?.long_short_ratio(query).await`
  - `sdk.market(exchange)?.taker_buy_sell_volume(query).await`
- 统一 account balances：
  - `sdk.account(exchange)?.balances().await`
- 统一账户交易设置：
  - `sdk.account(exchange)?.capabilities()`
  - `sdk.account(exchange)?.set_leverage(request).await`
  - `sdk.account(exchange)?.set_position_mode(request).await`
  - `sdk.account(exchange)?.set_symbol_margin_mode(request).await`
  - `sdk.account(exchange)?.ensure_order_margin_mode(request).await`
  - `sdk.account(exchange)?.prepare_order_settings(request).await`
- 统一 positions：
  - `sdk.positions(exchange)?.list(Some(&instrument)).await`
- 统一基础下单/撤单：
  - `sdk.trade(exchange)?.place_order(request).await`
  - `sdk.trade(exchange)?.cancel_order(request).await`
- 统一订单查询：
  - `sdk.orders(exchange)?.get(query).await`
  - `sdk.orders(exchange)?.open(query).await`
  - `sdk.orders(exchange)?.history(query).await`
- 统一成交明细查询：
  - `sdk.fills(exchange)?.list(query).await`
- 统一错误入口 `crypto_exc_all::Error`。
- `raw` 逃生口：
  - `crypto_exc_all::raw::okx`
  - `crypto_exc_all::raw::binance`
  - `crypto_exc_all::raw::bitget`

`crypto_exc_all::raw::bitget` 暴露 `bitget_rs` 的原生 V2 REST/WebSocket wrapper，覆盖 Bitget Futures market/account/trade、Spot wallet/asset、public notices、common trade-rate，以及 V2 public/private WebSocket URL、login、ping/pong、subscribe/unsubscribe、place-order/cancel-order trade helper、trade ack parser、ticker/orders/account/positions/books/trade/candle/fill typed event parser、运行中动态订阅/取消订阅、私有连接登录重放和 ack gate、入站消息超时重连和基础重连订阅重放。统一 facade 当前稳定暴露跨交易所 `ticker` / `orderbook` / `candles` / `funding rate` / `funding rate history` / `mark price` / `open interest` / `long-short ratio` / `taker buy-sell volume` / `balances` / `set leverage` / `set position mode` / `set symbol margin mode` / `ensure order margin mode` / `prepare order settings` / `positions` / `place_order` / `cancel_order` / `order detail` / `open orders` / `order history` / `fills`；不同交易所的账户配置语义通过 `capabilities()` 暴露，OKX 这类没有 symbol 级独立 margin-mode switch 的交易所会返回 `Unsupported`，策略层可优先使用 `prepare_order_settings` 一次性处理持仓模式、保证金模式和杠杆预配置。

## 测试

根 crate 包含外部调用场景集成测试：测试代码只引入 `crypto_exc_all`，通过 mock HTTP 同时调用 OKX、Binance 和 Bitget 的统一 ticker、orderbook、candles、funding rate、funding rate history、mark price、open interest、long-short ratio、taker buy-sell volume、balances、set leverage、set position mode、set symbol margin mode、ensure order margin mode、prepare order settings、positions、place_order、cancel_order、order detail、open orders、order history 和 fills 接口。

```bash
cargo test -p crypto_exc_all -- --nocapture
```

## 继续接入交易所

新增 Bybit、Hyperliquid 等交易所时，按 [Exchange Integration Playbook](docs/exchange-integration-playbook.md) 执行。该文档记录了本轮迭代沉淀下来的 crate 命名、dependency alias、adapter、测试、发布和安全检查流程。
