# crypto_exc_all

统一加密货币交易所 SDK facade。外部业务只依赖 `crypto_exc_all`，由根 crate 自动加载不同交易所的 API key，并在内部转成对应的 `okx_rs` / `binance_rs` / `bitget_rs` / `bybit_rs` / `gate_rs` / `hyperliquid_rs` client。

## 发布结构

本项目按三层发布：

- `okx_rs`: OKX 交易所 SDK crate。
- `binance_rs`: Binance USDⓈ-M Futures SDK crate。
- `bitget_rs`: Bitget V2 Futures SDK crate。
- `bybit_rs`: Bybit V5 SDK crate。
- `gate_rs`: Gate Futures SDK crate。
- `hyperliquid_rs`: Hyperliquid read-only `/info` SDK crate。
- `crypto_exc_all`: 统一入口 crate，通过版本依赖引用各交易所 SDK。

本地开发使用 `path + version` 依赖；发布时先发布子交易所 crate，再发布 `crypto_exc_all`。

```toml
[dependencies]
crypto_exc_all = "0.1"
```

当前根 crate 的默认 feature 会启用 OKX、Binance、Bitget、Bybit、Gate 和 Hyperliquid：

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
```


Bitget:

```env
BITGET_API_KEY=...
BITGET_API_SECRET=...
BITGET_PASSPHRASE=...
BITGET_PRODUCT_TYPE=USDT-FUTURES
BITGET_WS_PUBLIC_URL=wss://ws.bitget.com/v2/ws/public
BITGET_WS_PRIVATE_URL=wss://ws.bitget.com/v2/ws/private
```

`BITGET_PRODUCT_TYPE` 可省略，根 adapter 默认使用 `USDT-FUTURES`。也支持 `.env` 中的小写/既有混合大小写变量名：`bitget_api_key`、`bitget_api_secret`、`bitget_passphrase`、`bitget_PASSPHRASE`。
`BITGET_WS_PUBLIC_URL` / `BITGET_WS_PRIVATE_URL` 可省略，`bitget_rs` 会默认使用 Bitget V2 public/private WebSocket 主域名。

Bybit:

```env
BYBIT_API_KEY=...
BYBIT_API_SECRET=...
BYBIT_CATEGORY=linear
```

Gate:

```env
GATE_API_KEY=...
GATE_API_SECRET=...
GATE_SETTLE=usdt
```

Hyperliquid:

```env
HYPERLIQUID_ENABLED=1
HYPERLIQUID_USER_ADDRESS=0x...
HYPERLIQUID_API_URL=https://api.hyperliquid.xyz
```

`HYPERLIQUID_API_URL` 可省略，默认使用官方主域名。`HYPERLIQUID_USER_ADDRESS` 只用于 read-only `/info` 用户状态查询；不代表 SDK 已接入 `/exchange` live mutation。

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

- 自动读取 OKX / Binance / Bitget / Bybit / Gate 凭证；Hyperliquid 通过 `HYPERLIQUID_ENABLED` / `HYPERLIQUID_API_URL` / `HYPERLIQUID_USER_ADDRESS` 显式启用。
- `CryptoSdk::from_env()` / `CryptoSdk::from_config()`。
- `sdk.configured_exchanges()`。
- 统一 `Instrument`，自动映射交易所 symbol：
  - Binance 永续：`BTCUSDT`
  - OKX 永续：`BTC-USDT-SWAP`
  - Bitget USDT 永续：`BTCUSDT`
  - Bybit linear 永续：`BTCUSDT`
  - Gate USDT 永续：`BTC_USDT`
  - Hyperliquid perp：`BTC`
  - Hyperliquid spot：`PURR/USDC`；其他 spot market-data coin 会按官方 `spotMeta` 解析为 `@{index}`，例如 HYPE/USDC -> `@107`
- 统一 market ticker：
  - `sdk.market(exchange)?.ticker(&instrument).await`
  - `sdk.market(exchange)?.tickers(instrument_type).await`
- 统一 market orderbook 和 candles：
  - `sdk.market(exchange)?.orderbook(query).await`
  - `sdk.market(exchange)?.candles(query).await`
- 统一 derivatives market metrics：
  - `sdk.market(exchange)?.funding_rate(&instrument).await`
  - `sdk.market(exchange)?.funding_rate_history(query).await`
  - `sdk.market(exchange)?.mark_price(&instrument).await`
  - `sdk.market(exchange)?.open_interest(&instrument).await`
  - `sdk.market(exchange)?.open_interest_history(query).await`
- 统一 market sentiment stats：
  - `sdk.market(exchange)?.long_short_ratio(query).await`
  - `sdk.market(exchange)?.top_trader_position_ratio(query).await`
  - `sdk.market(exchange)?.taker_buy_sell_volume(query).await`
  - `top_trader_position_ratio` 当前只接入 Binance / OKX 官方 top trader position ratio；其他交易所返回 `Unsupported`。
- 统一 account balances：
  - `sdk.account(exchange)?.balances().await`
- 统一 account bills / 资金流水：
  - `sdk.account(exchange)?.bills(query).await`
  - OKX 映射 account bills；Binance 映射 deposit / withdrawal / universal transfer history；Bitget 映射 mix account bills；Bybit 映射 transfer / deposit / withdrawal records；Gate 映射 futures account book；Hyperliquid 映射 `userFunding` + `userNonFundingLedgerUpdates`。交易所不支持的过滤条件返回 `Unsupported`，不静默忽略。
- 统一 platform events / 交易所公告与系统状态：
  - `sdk.platform(exchange)?.system_status(query).await`
  - `sdk.platform(exchange)?.announcements(query).await`
  - 当前 Bybit / OKX 已映射 system status 和 announcements；Binance / Bitget 已映射 announcements。未接入交易所返回 `Unsupported`，不伪造空列表。
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
  - `sdk.trade(exchange)?.capabilities()`
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
  - `crypto_exc_all::raw::bybit`
  - `crypto_exc_all::raw::gate`
  - `crypto_exc_all::raw::hyperliquid`

## 契约边界

`crypto_exc_all` 是交易所协议与能力聚合层，不承载用户授权、实盘门禁、强制止损、余额风控、worker lease 或策略状态机。生产实盘 mutation 应由 `rust_quant` 的 Core Gateway / Core worker 负责，SDK 只保证请求能被正确签名、序列化、映射和提交。

SDK 不会判断业务上是否必须带止损；但调用方传入的字段必须被忠实表达。如果 adapter 无法把某个字段等价映射到底层交易所 API，必须返回 `Error::Unsupported` 或明确错误，不允许静默丢字段或降级提交。

订单保护相关能力：

| 交易所 | `attached_stop_loss_price` on `place_order` | `place_protective_order` |
|---|---:|---:|
| OKX | 支持 | 不支持 |
| Binance | 不支持 | 支持 |
| Bitget | 支持 | 不支持 |
| Bybit | 不支持 | 不支持 |
| Gate | 不支持 | 不支持 |
| Hyperliquid | 不支持 | 不支持 |

调用侧可以先读取 `sdk.trade(exchange)?.capabilities()`。例如 Binance 支持独立 protective order，但不支持在 `place_order` 请求中携带 `attached_stop_loss_price`；如果传入该字段，SDK 会直接返回 `Unsupported`，不会提交裸主单。

Hyperliquid 当前不使用社区 SDK，只按官方 HTTP API 接入当前需要的 read-only `/info`：perp `perpDexs`、`meta`、`metaAndAssetCtxs`、`allMids`、`l2Book`、`candleSnapshot`、`fundingHistory`、`clearinghouseState`、`userFunding`、`userNonFundingLedgerUpdates`、`predictedFundings`、`perpsAtOpenInterestCap`、`perpDeployAuctionStatus`、`activeAssetData`、`perpDexLimits`、`perpDexStatus`、`allPerpMetas`、`perpAnnotation`、`perpCategories`、`perpConciseAnnotations`、`openOrders`、`frontendOpenOrders`、`orderStatus`、`historicalOrders`、`userTwapSliceFills`、`userFills` / `userFillsByTime`，其中 `meta`、`metaAndAssetCtxs`、`clearinghouseState`、`allMids`、`openOrders`、`frontendOpenOrders`、`perpsAtOpenInterestCap` 支持官方 `dex` 选项，`l2Book` 支持官方 `nSigFigs` / `mantissa` 聚合参数，fills 支持官方 `aggregateByTime` 选项；spot `spotMeta`、`spotMetaAndAssetCtxs`、`spotClearinghouseState`、`spotDeployState`、`spotPairDeployAuctionStatus`、`tokenDetails`、`outcomeMeta`、`settledOutcome`；以及 readiness/raw evidence `userRateLimit`、`userRole`、`userFees`、`referral`、`delegations`、`delegatorSummary`、`delegatorHistory`、`delegatorRewards`、`subAccounts`、`portfolio`、`vaultDetails`、`userVaultEquities`、`userAbstraction`、`userDexAbstraction`、`borrowLendUserState`、`borrowLendReserveState`、`allBorrowLendReserveStates`、`approvedBuilders`、`maxBuilderFee`。`/exchange` 下单、撤单、改杠杆等 mutation 尚未接入统一 facade；调用这些统一交易接口会返回 `Unsupported`。

`crypto_exc_all::raw::bitget` 暴露 `bitget_rs` 的原生 V2 REST/WebSocket wrapper，覆盖 Bitget Futures market/account/trade、Spot wallet/asset、public notices、common trade-rate，以及 V2 public/private WebSocket URL、login、ping/pong、subscribe/unsubscribe、place-order/cancel-order trade helper、trade ack parser、ticker/orders/account/positions/books/trade/candle/fill typed event parser、运行中动态订阅/取消订阅、私有连接登录重放和 ack gate、入站消息超时重连、连接内失败重连次数限制和基础重连订阅重放。`crypto_exc_all::raw::hyperliquid` 暴露 Hyperliquid read-only `/info` wrapper。统一 facade 当前稳定暴露跨交易所 `ticker` / `orderbook` / `candles` / `funding rate` / `funding rate history` / `mark price` / `open interest` / `open interest history` / `long-short ratio` / `top trader position ratio` / `taker buy-sell volume` / `balances` / `set leverage` / `set position mode` / `set symbol margin mode` / `ensure order margin mode` / `prepare order settings` / `positions` / `place_order` / `cancel_order` / `order detail` / `open orders` / `order history` / `fills`；不同交易所的账户配置语义通过 `capabilities()` 暴露，OKX 这类没有 symbol 级独立 margin-mode switch 的交易所会返回 `Unsupported`，策略层可优先使用 `prepare_order_settings` 一次性处理持仓模式、保证金模式和杠杆预配置。

## 测试

根 crate 包含外部调用场景集成测试：测试代码只引入 `crypto_exc_all`，通过 mock HTTP 同时调用 OKX、Binance、Bitget、Bybit、Gate 和 Hyperliquid 的统一 ticker、orderbook、candles、funding rate、funding rate history、mark price、open interest、open interest history、long-short ratio、top trader position ratio、taker buy-sell volume、balances、account bills、platform events、set leverage、set position mode、set symbol margin mode、ensure order margin mode、prepare order settings、positions、place_order、cancel_order、order detail、open orders、order history 和 fills 接口；其中 sentiment stats、platform events 和 Hyperliquid mutation 仅在交易所有官方接口且已接入时暴露，否则返回 `Unsupported`。

```bash
cargo test -p crypto_exc_all -- --nocapture
```

## 继续接入交易所
新增 Bybit、Hyperliquid 等交易所时，按 [Exchange Integration Playbook](docs/exchange-integration-playbook.md) 执行。该文档记录了本轮迭代沉淀下来的 crate 命名、dependency alias、adapter、测试、发布和安全检查流程。
