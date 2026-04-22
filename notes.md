# Notes: Binance Futures SDK Architecture

## Sources

### Project README

- File: `README.md`
- Key points:
  - 项目目标是多交易所 Rust SDK 集合，先补 Binance、Bitget、Bybit、Hyperliquid。
  - OKX 已有部分实现，应先梳理 OKX 接口，再按其他交易所最新 API 补齐。
  - 后续需要统一 SDK 接口、字段、返回值和错误码，并覆盖单元测试、流程测试、正常/异常/边界场景。

### OKX Local Implementation

- Files: `okx_rs/src/{config.rs,client.rs,api,dto,error.rs}`
- Key points:
  - OKX 是独立 crate，核心结构是 `config`、`client`、`api/*`、`dto/*`、`error`。
  - `OkxClient` 统一负责鉴权头、签名、HTTP 请求和错误解析。
  - API 按业务域拆分：account、asset、trade、market、public_data、big_data、announcements、websocket。
  - DTO 类型主要按业务域放在 `dto/*` 中。

### Binance Official USDⓈ-M Futures Docs

- URL: https://developers.binance.com/docs/derivatives/usds-margined-futures/general-info
- Key points:
  - REST 生产基础地址是 `https://fapi.binance.com`。
  - Testnet REST 地址是 `https://demo-fapi.binance.com`。
  - API key 通过 `X-MBX-APIKEY` 请求头发送。
  - `TRADE` 和 `USER_DATA` 端点需要 `timestamp` 和 HMAC-SHA256 `signature`。
  - `recvWindow` 可选，默认 5000ms，官方建议使用 5000ms 或更小。
  - 错误响应形态是 `{ "code": -1121, "msg": "Invalid symbol." }`。
  - 文档 SDK 段主要列 Python3 和 Java；免责声明提示 SDK 由合作方/用户提供或辅助熟悉接口。

### Binance Rust Connector

- URL: https://github.com/binance/binance-connector-rust
- Crate: `binance-sdk = 45.0.0`
- Key points:
  - Binance GitHub 提供官方 Rust connector，自动生成，支持 `derivatives_trading_usds_futures` feature。
  - 需要 Rust 1.86+；当前本机 `rustc 1.89.0` 满足。
  - 官方 connector 适合作为底层可选后端，但直接暴露会与当前 OKX 手写 SDK 风格、统一 DTO/错误目标不一致。

### Binance Wallet/SAPI Docs

- Coin information: https://developers.binance.com/docs/wallet/capital/all-coins-info
- Funding wallet: https://developers.binance.com/docs/wallet/asset/funding-wallet
- User assets: https://developers.binance.com/docs/wallet/asset/user-assets
- Key points:
  - Wallet/SAPI 接口生产基础地址是 `https://api.binance.com`，与 USDⓈ-M Futures `https://fapi.binance.com` 分离。
  - USER_DATA/TRADE 类型接口仍使用 `X-MBX-APIKEY`、`timestamp`、`recvWindow` 和 HMAC-SHA256 query signature。
  - 资产域需要覆盖币种信息、账户资产、资金账户、划转、充值地址、充值记录、提现记录和提现申请。

### Binance Announcements and WebSocket Docs

- Announcement BAPI discussion: https://dev.binance.vision/t/announcement-related-api/33478
- User data stream start: https://developers.binance.com/docs/derivatives/usds-margined-futures/user-data-streams/Start-User-Data-Stream
- User data stream keepalive: https://developers.binance.com/docs/derivatives/usds-margined-futures/user-data-streams/Keepalive-User-Data-Stream
- User data stream close: https://developers.binance.com/docs/derivatives/usds-margined-futures/user-data-streams/Close-User-Data-Stream
- Important WebSocket change notice: https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/Important-WebSocket-Change-Notice
- Key points:
  - Binance Developer Community states `bapi` endpoints are not public endpoints; announcement wrapper is therefore best-effort and should be monitored separately in production.
  - USDⓈ-M Futures listenKey REST endpoints are `POST` / `PUT` / `DELETE /fapi/v1/listenKey` and require API-key authentication without HMAC query signing.
  - Binance introduced split WebSocket routes under `/public`, `/market`, and `/private`; legacy `/ws` and `/stream` are scheduled to be removed after 2026-04-23.

### Bitget V2 Futures Docs

- Ticker: https://www.bitget.com/api-doc/contract/market/Get-Ticker
- Account list: https://www.bitget.com/api-doc/classic/contract/account/Get-Account-List
- Signature: https://www.bitget.com/api-doc/common/signature
- Contract config: https://www.bitget.com/api-doc/contract/market/Get-All-Symbols-Contracts
- Candles: https://www.bitget.com/api-doc/contract/market/Get-Candle-Data
- Long/short ratio: https://www.bitget.com/api-doc/classic/common/apidata/Long-Short
- Account long/short ratio: https://www.bitget.com/api-doc/common/apidata/Account-Long-Short
- Taker buy/sell volume: https://www.bitget.com/api-doc/common/apidata/Taker-Buy-Sell
- Account bills: https://www.bitget.com/api-doc/contract/account/Get-Account-Bill
- Positions: https://www.bitget.com/api-doc/contract/position/get-all-position
- Place order: https://www.bitget.com/api-doc/contract/trade/Place-Order
- Order history: https://www.bitget.com/api-doc/contract/trade/Get-Orders-History
- Wallet transfer: https://www.bitget.com/api-doc/spot/account/Wallet-Transfer
- Withdraw: https://www.bitget.com/api-doc/spot/wallet/Wallet-Withdrawal
- Announcements: https://www.bitget.com/api-doc/common/notice/Get-All-Notices
- Key points:
  - Futures ticker endpoint is `GET /api/v2/mix/market/ticker` with required `productType` and `symbol`.
  - Futures account list endpoint is signed `GET /api/v2/mix/account/accounts` with required `productType`.
  - Bitget V2 signed REST requests use `ACCESS-KEY`, `ACCESS-SIGN`, `ACCESS-TIMESTAMP`, `ACCESS-PASSPHRASE`, `locale`, and JSON content headers.
  - HMAC signature payload is `timestamp + METHOD + requestPath + (?queryString) + body`, encoded with Base64.
  - USDT perpetual product type is `USDT-FUTURES`; root adapter defaults to it for `Instrument::perp(...)`.
- Local `.env` currently uses `bitget_PASSPHRASE`; SDK config accepts that name in addition to `BITGET_PASSPHRASE` and all-lowercase variants.

## Architecture Findings

- 根 crate `crypto_exc_all` 目前只是空壳二进制，`src/main.rs` 仅打印 `Hello, world!`。
- `binance_rs`、`bitget_rs`、`bybit_rs`、`hyperliquid_rs` 目录为空。
- `.env` 中存在 `binance_api_key` 和 `binance_api_secret`，实现应同时支持小写现状和常见大写 `BINANCE_API_KEY` / `BINANCE_API_SECRET`。
- 本阶段不应下单；优先选择只读的 Binance account balance，因为它能验证 `.env` 密钥、签名、请求头、错误处理，又没有交易副作用。

## SDK Strategy Decision

- 第一阶段采用与 OKX 一致的轻量本地 SDK 结构，在 `binance_rs` 内实现 `config`、`client`、`api`、`dto`、`error`、`utils`。
- 不直接依赖 `binance-sdk` 作为业务 API，因为当前项目最终目标是统一多交易所接口；手写薄封装更容易统一命名、DTO、错误和测试。
- 保留官方 `binance-sdk` 作为后续可选适配层：如果未来需要大量端点快速覆盖，可在 `binance_rs` 内部包一层 adapter，不让自动生成类型泄漏到聚合 SDK 公共接口。
- 第一项交付：Binance USDⓈ-M Futures `GET /fapi/v2/balance`，附带公共 `GET /fapi/v1/time` 用于连通性和时间戳校验。

## Binance REST Coverage Added

### Market Data

- `GET /fapi/v1/time`
- `GET /fapi/v1/exchangeInfo`
- `GET /fapi/v1/depth`
- `GET /fapi/v1/klines`
- `GET /fapi/v1/ticker/24hr`
- `GET /fapi/v1/fundingRate`
- `GET /fapi/v1/premiumIndex`
- `GET /fapi/v1/openInterest`
- `GET /futures/data/openInterestHist`
- `GET /futures/data/topLongShortPositionRatio`
- `GET /futures/data/topLongShortAccountRatio`
- `GET /futures/data/globalLongShortAccountRatio`
- `GET /futures/data/takerlongshortRatio`

### Account and Position

- `GET /fapi/v2/balance`
- `GET /fapi/v3/account`
- `GET /fapi/v3/positionRisk`
- `GET /fapi/v1/income`
- `GET /fapi/v1/commissionRate`
- `GET /fapi/v1/accountConfig`
- `GET /fapi/v1/symbolConfig`
- `GET /fapi/v1/rateLimit/order`
- `GET /fapi/v1/leverageBracket`
- `GET /fapi/v1/multiAssetsMargin`
- `GET /fapi/v1/positionSide/dual`

### Trade and Order

- `POST /fapi/v1/order`
- `POST /fapi/v1/batchOrders`
- `POST /fapi/v1/order/test`
- `PUT /fapi/v1/order`
- `PUT /fapi/v1/batchOrders`
- `DELETE /fapi/v1/order`
- `DELETE /fapi/v1/batchOrders`
- `DELETE /fapi/v1/allOpenOrders`
- `GET /fapi/v1/order`
- `GET /fapi/v1/openOrder`
- `GET /fapi/v1/openOrders`
- `GET /fapi/v1/allOrders`
- `GET /fapi/v1/userTrades`
- `POST /fapi/v1/leverage`
- `POST /fapi/v1/marginType`
- `POST /fapi/v1/positionSide/dual`
- `POST /fapi/v1/multiAssetsMargin`
- `POST /fapi/v1/positionMargin`

### Asset and Wallet

- `GET /sapi/v1/capital/config/getall`
- `GET /sapi/v1/asset/wallet/balance`
- `POST /sapi/v3/asset/getUserAsset`
- `POST /sapi/v1/asset/get-funding-asset`
- `POST /sapi/v1/asset/transfer`
- `GET /sapi/v1/asset/transfer`
- `GET /sapi/v1/capital/deposit/address`
- `GET /sapi/v1/capital/deposit/hisrec`
- `GET /sapi/v1/capital/withdraw/history`
- `POST /sapi/v1/capital/withdraw/apply`

### Announcements

- `GET /bapi/composite/v1/public/cms/article/list/query`

### WebSocket Support APIs

- `POST /fapi/v1/listenKey`
- `PUT /fapi/v1/listenKey`
- `DELETE /fapi/v1/listenKey`
- URL builders for split `wss://fstream.binance.com/public`, `/market`, and `/private` stream routes.
- WebSocket JSON receive loop with ping/pong handling.
- `SUBSCRIBE` / `UNSUBSCRIBE` request helpers.
- Basic reconnect manager that reconnects and replays stored subscriptions.
- WebSocket SOCKS5/SOCKS5h proxy connection path so stream clients can reuse the same proxy configuration as REST.
- Connection state and health metrics: disconnected/connecting/connected/reconnecting/stopped, connection attempts, reconnects, last message time, last error.
- Split-route hub for Binance's `/public`, `/market`, and `/private` WebSocket migration model.
- Typed parsers for private user-data events `listenKeyExpired`, `MARGIN_CALL`, `ORDER_TRADE_UPDATE`, `TRADE_LITE`, `ACCOUNT_UPDATE`, `ACCOUNT_CONFIG_UPDATE`, `STRATEGY_UPDATE`, `GRID_UPDATE`, `CONDITIONAL_ORDER_TRIGGER_REJECT`, and `ALGO_UPDATE`, with raw fallback for other events.

### Remaining Enhancement Gaps

- A few exchange-specific public data endpoints without a direct USD-M Futures REST equivalent.

## Live Order Attempt

- Added explicit proxy support in `Config`/`BinanceClient`, including `BINANCE_PROXY_URL` and fallback to global proxy variables.
- Current shell proxy `socks5://127.0.0.1:7897` is normalized to `socks5h://127.0.0.1:7897`, which allows public Binance Futures endpoints to respond.
- Added `examples/live_post_only_order.rs` for controlled real-order verification:
  - Reads real `exchangeInfo` and 24h ticker.
  - Builds a minimum-size `LIMIT + GTX` post-only order.
  - Sends real `POST /fapi/v1/order`.
  - Immediately cancels with `DELETE /fapi/v1/order` if Binance returns an `orderId`.
- Live mainnet order attempt reached Binance but failed before order creation with `-2015` (`Invalid API-key, IP, or permissions for action`). No order ID was created.
- After the IP whitelist was updated, the same live path progressed:
  - BTCUSDT failed with `-4061` until `BINANCE_LIVE_POSITION_SIDE=LONG` was added for Hedge Mode.
  - BTCUSDT with `positionSide=LONG` then failed with `-2019` because available margin was insufficient for the minimum BTC order.
  - DOGEUSDT succeeded with a real `LIMIT + GTX` post-only order, then immediately canceled the returned order. Binance returned `status=NEW` on placement and `status=CANCELED` on cancel, with `executedQty=0`.

## Asset/SAPI Implementation Notes

- Added `BINANCE_SAPI_API_URL` / `binance_sapi_api_url` so Futures REST and Wallet/SAPI REST can use distinct base URLs.
- Added `BinanceAsset` wrapper under `api::asset`, reusing the existing signed-request client and error model.
- Request builders intentionally mirror the existing trade/account style: mandatory fields in `new(...)`, optional query fields via `with_*` methods.
- The wallet/asset tests mock signed SAPI endpoints instead of calling real transfer or withdrawal APIs. Real movement of funds should remain explicit and separate from normal verification.

## Announcements/WebSocket Implementation Notes

- Added `BINANCE_WEB_API_URL` / `binance_web_api_url` for Binance website BAPI announcement reads.
- Added `BINANCE_WS_STREAM_URL` / `binance_ws_stream_url` for WebSocket URL builders, defaulting to `wss://fstream.binance.com`.
- Added `BinanceAnnouncements` with `AnnouncementListRequest`; `latest()` uses catalog `48`, matching the Binance community guidance for latest announcements.
- Added `BinanceWebsocket` listenKey wrappers, split-route URL builders, a socket session reader, subscription helpers, and `BinanceWebsocketManager` for basic reconnect with subscription replay.
- Added `examples/user_stream_listen_key.rs` and verified it against Binance mainnet: the example created a listenKey, printed only a masked value, then closed the stream successfully.
- Added `examples/websocket_public_stream.rs` for public-market WebSocket smoke testing. It connects to the configured stream URL through SOCKS5/SOCKS5h proxy when configured, waits for one JSON message, prints it, then closes. Verified against Binance mainnet with `btcusdt@aggTrade`; the example received a combined-stream JSON payload and exited successfully.
- Added `BinanceWebsocketHub` to split subscriptions by route and run separate managers for `/public`, `/market`, and `/private`.
- Added `WebsocketMetrics` and `ConnectionState` to make reconnect health observable by SDK users.
- Added `BinanceWebsocketEvent::parse` with typed structs for the current USDⓈ-M Futures private user-data events: listen-key expiry, margin call, order update, trade lite, account update, account configuration, strategy update, grid update, conditional trigger reject, and algo update. The parser also unwraps combined-stream `{stream, data}` payloads before typed dispatch.
- Hub route URLs that already embed `?streams=` or `listenKey=` do not send an additional JSON `SUBSCRIBE`; this matches the combined-stream mode used by the live examples and avoids early disconnects on Binance's split routes.
- Added `examples/websocket_split_hub.rs` and verified it against Binance mainnet through the configured proxy. The example connected via split-route combined URLs and received live `btcusdt@aggTrade` payloads.

## Bitget Aggregation Implementation Notes

- Added `bitget_rs` as a workspace member and optional root dependency behind the `bitget` feature.
- `bitget_rs` includes `Config`, `Credentials`, `BitgetClient`, public request dispatch, signed request dispatch, signed JSON body dispatch, Bitget API envelope parsing, and V2 wrappers.
- Root `crypto_exc_all` now includes `ExchangeId::Bitget`, `BitgetExchangeConfig`, environment lookup for uppercase and lowercase Bitget credentials, `Instrument` symbol mapping, error conversion, and `src/adapters/bitget.rs`.
- Root adapter maps Bitget ticker fields `lastPr`, `bidPr`, `askPr`, `quoteVolume/baseVolume`, and `ts` into unified `Ticker`.
- Root adapter maps Bitget account fields `marginCoin`, `accountEquity/usdtEquity`, `available`, and `locked` into unified `Balance`.
- External consumer test now uses only `crypto_exc_all` to call Binance, OKX, and Bitget tickers through mock HTTP servers.
- Live Bitget diagnosis: the configured key/passphrase are loaded from `.env` without outer whitespace and authenticate far enough on UTA V3 to receive `40084` account-mode feedback, but every tested V2 signed endpoint returns `40012`. Treat this as a Bitget API key/account-mode mismatch until a Classic V2-compatible key is supplied or the account is moved to the API family being called.
- After replacing the Bitget API key/passphrase and adding the current egress IP to the whitelist, V2 signed account, position, and trade-rate reads returned `00000`. A real `post_only` BTCUSDT futures order attempt reached trade risk checks and failed with `40762` because both USDT-Futures and spot USDT available balances were `0`; no orderId was created, so no cancel was needed.

## Bitget REST Coverage Added

### Public and Market Data

- `GET /api/v2/public/time`
- `GET /api/v2/public/annoucements`
- `GET /api/v2/mix/market/ticker`
- `GET /api/v2/mix/market/tickers`
- `GET /api/v2/mix/market/contracts`
- `GET /api/v2/mix/market/orderbook`
- `GET /api/v2/mix/market/merge-depth`
- `GET /api/v2/mix/market/candles`
- `GET /api/v2/mix/market/history-candles`
- `GET /api/v2/mix/market/symbol-price`
- `GET /api/v2/mix/market/current-fund-rate`
- `GET /api/v2/mix/market/history-fund-rate`
- `GET /api/v2/mix/market/open-interest`
- `GET /api/v2/mix/market/oi-limit`
- `GET /api/v2/mix/market/query-position-lever`
- `GET /api/v2/mix/market/long-short`
- `GET /api/v2/mix/market/account-long-short`
- `GET /api/v2/mix/market/taker-buy-sell`
- `GET /api/v2/mix/market/exchange-rate`

## Root Unified Facade Notes

- 根 crate `crypto_exc_all` 继续保持外部只依赖一个 crate 的使用方式：`CryptoSdk::from_env()` 根据 `.env` 自动装配 OKX/Binance/Bitget。
- 当前稳定统一 DTO：
  - `Ticker`：市场最新价、买卖一、24h 量、时间戳。
  - `OrderBook`：交易所、统一 `Instrument`、交易所 symbol、买卖盘档位、时间戳、raw。
  - `Candle`：交易所、统一 `Instrument`、交易所 symbol、开/收盘时间、OHLC、成交量、计价成交量、是否确认、raw。
  - `FundingRate`：交易所、统一 `Instrument`、交易所 symbol、资金费率、本期/下一期资金费时间、下一期预测费率、标记价、raw。
  - `MarkPrice`：交易所、统一 `Instrument`、交易所 symbol、标记价、指数价、资金费率、下一期资金费时间、时间戳、raw。
  - `OpenInterest`：交易所、统一 `Instrument`、交易所 symbol、未平仓量、未平仓价值、时间戳、raw。
  - `LongShortRatio`：交易所、统一 `Instrument`、交易所 symbol、周期、多空比、多/空侧比例、时间戳、raw。
  - `TakerBuySellVolume`：交易所、统一 `Instrument`、交易所 symbol、周期、主动买入量、主动卖出量、买卖比、时间戳、raw。
  - `Balance`：资产、总额、可用、冻结。
  - `LeverageSetting`：交易所、统一 `Instrument`、交易所 symbol、杠杆、保证金模式、保证金币种、持仓方向、raw。
  - `PositionModeSetting`：交易所、统一持仓模式、交易所原始模式值、产品类型、raw。
  - `Position`：交易所、统一 `Instrument`、交易所 symbol、方向、数量、开仓价、标记价、未实现盈亏、杠杆、保证金模式、预估强平价、raw。
  - `OrderAck`：交易所、统一 `Instrument`、交易所 symbol、订单 ID、客户订单 ID、状态、raw。
  - `Order`：交易所、统一 `Instrument`、交易所 symbol、订单 ID、客户订单 ID、方向、订单类型、价格、数量、已成交数量、成交均价、状态、创建/更新时间、raw。
  - `Fill`：交易所、统一 `Instrument`、交易所 symbol、成交 ID、订单 ID、方向、成交价、成交数量、手续费、手续费币种、maker/taker 角色、时间戳、raw。
  - `AccountCapabilities`：账户 facade 支持能力标记，包括 `set_leverage`、`set_position_mode`、`set_symbol_margin_mode`、`order_level_margin_mode`。
  - `MarginMode`：统一保证金模式枚举，覆盖 `Cross`、`Isolated`，并保留 `Raw` 逃生值兼容交易所特有字符串。
  - `SymbolMarginModeSetting`：交易所、统一 `Instrument`、交易所 symbol、统一 margin mode、原始 margin mode、product type、margin coin、raw。
  - `EnsureOrderMarginModeResult`：交易所、统一 `Instrument`、交易所 symbol、统一 margin mode、实际应用方式、原始 margin mode、product type、margin coin、raw。
  - `PrepareOrderSettingsResult`：交易所、统一 `Instrument`、交易所 symbol，以及可选的 position mode、margin mode、leverage 三个预配置结果。
- 当前稳定统一入口：
  - `sdk.market(exchange)?.ticker(&instrument).await`
  - `sdk.market(exchange)?.orderbook(query).await`
  - `sdk.market(exchange)?.candles(query).await`
  - `sdk.market(exchange)?.funding_rate(&instrument).await`
  - `sdk.market(exchange)?.funding_rate_history(query).await`
  - `sdk.market(exchange)?.mark_price(&instrument).await`
  - `sdk.market(exchange)?.open_interest(&instrument).await`
  - `sdk.market(exchange)?.long_short_ratio(query).await`
  - `sdk.market(exchange)?.taker_buy_sell_volume(query).await`
  - `sdk.account(exchange)?.balances().await`
  - `sdk.account(exchange)?.capabilities()`
  - `sdk.account(exchange)?.set_leverage(request).await`
  - `sdk.account(exchange)?.set_position_mode(request).await`
  - `sdk.account(exchange)?.set_symbol_margin_mode(request).await`
  - `sdk.account(exchange)?.ensure_order_margin_mode(request).await`
  - `sdk.account(exchange)?.prepare_order_settings(request).await`
  - `sdk.positions(exchange)?.list(Some(&instrument)).await`
  - `sdk.trade(exchange)?.place_order(request).await`
  - `sdk.trade(exchange)?.cancel_order(request).await`
  - `sdk.orders(exchange)?.get(query).await`
  - `sdk.orders(exchange)?.open(query).await`
  - `sdk.orders(exchange)?.history(query).await`
  - `sdk.fills(exchange)?.list(query).await`
- 统一 `PlaceOrderRequest` 只放三家交易所都能稳定映射的基础字段：side、order_type、size、price、margin_mode、margin_coin、position_side、trade_side、client_order_id、reduce_only、time_in_force。
- 统一 `OrderQuery` / `OrderListQuery` 覆盖三家可稳定对齐的订单详情、当前挂单和历史订单查询；不同交易所不支持的列表过滤字段不会强行写入请求。
- 统一 `FillListQuery` 覆盖成交明细查询；其中 `after`/`before` 在不同交易所含义不完全一致，adapter 只映射对应交易所支持的游标字段。
- 统一 `OrderBookQuery` / `CandleQuery` 覆盖市场深度和 K 线；K 线 interval 原样传给交易所，调用方需要使用该交易所支持的粒度字符串。
- 统一 `FundingRateQuery` 覆盖历史资金费率查询；Binance 映射 `startTime/endTime/limit`，OKX 映射 `before/after/limit`，Bitget 当前 wrapper 只稳定映射 symbol/productType，无法对齐的分页字段不强行传入。
- 统一 `MarketStatsQuery` 覆盖市场统计周期与分页；Binance 映射 `period/startTime/endTime/limit`，OKX 映射 `period/begin/end/limit`，Bitget 当前 long-short/taker wrapper 只稳定映射 `period` 和 symbol。
- 统一 `SetLeverageRequest` 覆盖三家都可稳定设置的 symbol leverage；Binance 只映射 `symbol/leverage`，OKX 合约类 instrument 映射 `instId/lever/mgnMode/posSide` 且不发送 `ccy`，Bitget 映射 `symbol/productType/marginCoin/leverage`。
- 统一 `SetPositionModeRequest` 覆盖三家都可稳定设置的账户/产品级持仓模式；统一 `PositionMode::OneWay/Hedge` 分别映射 Binance `dualSidePosition=false/true`、OKX `net_mode/long_short_mode`、Bitget `one_way_mode/hedge_mode`。
- 统一 `SetSymbolMarginModeRequest` 只表达 symbol/product 级独立保证金模式切换：Binance 映射 `POST /fapi/v1/marginType`，Bitget 映射 `/api/v2/mix/account/set-margin-mode`，OKX 因没有完全等价的独立接口而返回 `Error::Unsupported { capability: "set_symbol_margin_mode" }`。OKX 保证金模式仍通过下单 `tdMode` 或设置杠杆时的 `mgnMode` 表达。
- 统一 `EnsureOrderMarginModeRequest` 给策略层使用：Binance/Bitget 通过 symbol 配置接口实际切换并返回 `MarginModeApplyMethod::SymbolConfiguration`；OKX 不发账户配置请求，返回 `MarginModeApplyMethod::OrderLevel`，调用方后续下单继续通过 `PlaceOrderRequest::with_margin_mode(...)` 写入 `tdMode`。
- 统一 `PrepareOrderSettingsRequest` 给策略层做下单前预配置：按 position mode、margin mode、leverage 的顺序执行；所有字段都是可选的，调用方可以只准备其中一部分。Binance/Bitget 的 margin mode 会真实切换 symbol 配置，OKX 的 margin mode 只返回 order-level 结果，leverage 设置仍带 `mgnMode`。
- 交易所高级能力继续走 `crypto_exc_all::raw::*`：批量下单、改单、全部撤单、计划单、资产划转、提现、WebSocket 特有事件等。
- 映射约定：
  - Binance post-only 使用 `LIMIT + timeInForce=GTX`。
  - OKX post-only 使用 `ordType=post_only`。
  - Bitget post-only 使用 `force=post_only`。
  - OKX 保证金模式统一输入 `cross`/`crossed` 都转 `cross`；Bitget 统一输入 `cross`/`crossed` 都转 `crossed`。
  - Binance 订单详情/历史分别映射 `GET /fapi/v1/order`、`GET /fapi/v1/allOrders`，当前挂单映射 `GET /fapi/v1/openOrders`。
  - OKX 订单详情/历史/当前挂单分别映射 `/api/v5/trade/order`、`/api/v5/trade/orders-history`、`/api/v5/trade/orders-pending`。
  - Bitget 订单详情/历史/当前挂单分别映射 `/api/v2/mix/order/detail`、`/api/v2/mix/order/orders-history`、`/api/v2/mix/order/orders-pending`。
  - 成交明细分别映射 Binance `GET /fapi/v1/userTrades`、OKX `/api/v5/trade/fills`、Bitget `/api/v2/mix/order/fills`。
  - 市场深度分别映射 Binance `GET /fapi/v1/depth`、OKX `/api/v5/market/books`、Bitget `/api/v2/mix/market/orderbook`。
  - K 线分别映射 Binance `GET /fapi/v1/klines`、OKX `/api/v5/market/candles`、Bitget `/api/v2/mix/market/candles`。
  - 当前资金费率分别映射 Binance `GET /fapi/v1/premiumIndex`、OKX `/api/v5/public/funding-rate`、Bitget `/api/v2/mix/market/current-fund-rate`。
  - 历史资金费率分别映射 Binance `GET /fapi/v1/fundingRate`、OKX `/api/v5/public/funding-rate-history`、Bitget `/api/v2/mix/market/history-fund-rate`。
  - 标记价分别映射 Binance `GET /fapi/v1/premiumIndex`、OKX `/api/v5/public/mark-price`、Bitget `/api/v2/mix/market/symbol-price`。
  - 未平仓量分别映射 Binance `GET /fapi/v1/openInterest`、OKX `/api/v5/public/open-interest`、Bitget `/api/v2/mix/market/open-interest`。
  - 多空比分别映射 Binance `GET /futures/data/globalLongShortAccountRatio`、OKX `/api/v5/rubik/stat/contracts/long-short-account-ratio-contract-top-trader`、Bitget `/api/v2/mix/market/account-long-short`。
  - 主动买卖量分别映射 Binance `GET /futures/data/takerlongshortRatio`、OKX `/api/v5/rubik/stat/taker-volume-contract`、Bitget `/api/v2/mix/market/taker-buy-sell`。
  - 设置杠杆分别映射 Binance `POST /fapi/v1/leverage`、OKX `/api/v5/account/set-leverage`、Bitget `/api/v2/mix/account/set-leverage`。
  - 设置持仓模式分别映射 Binance `POST /fapi/v1/positionSide/dual`、OKX `/api/v5/account/set-position-mode`、Bitget `/api/v2/mix/account/set-position-mode`。
  - 设置 symbol 保证金模式分别映射 Binance `POST /fapi/v1/marginType`、Bitget `/api/v2/mix/account/set-margin-mode`；OKX 返回 unsupported，调用方改用下单 `margin_mode`/`tdMode` 或 `set_leverage` 的 `mgnMode`。
  - 确保订单保证金模式分别复用 Binance/Bitget 的 symbol 保证金模式接口；OKX 返回 order-level 应用结果，不调用不存在的账户配置接口。
  - 准备下单配置复用上述三个账户配置能力，不新增交易所原生 API：先设持仓模式，再确保保证金模式，最后设置杠杆。

### Account, Position, and Fees

- `GET /api/v2/mix/account/accounts`
- `GET /api/v2/mix/account/account`
- `GET /api/v2/mix/account/bill`
- `GET /api/v2/mix/position/all-position`
- `GET /api/v2/common/trade-rate`
- `POST /api/v2/mix/account/set-leverage`
- `POST /api/v2/mix/account/set-margin-mode`
- `POST /api/v2/mix/account/set-position-mode`
- `POST /api/v2/mix/account/set-margin`
- `POST /api/v2/mix/account/set-asset-mode`

### Trade and Orders

- `POST /api/v2/mix/order/place-order`
- `POST /api/v2/mix/order/batch-place-order`
- `POST /api/v2/mix/order/cancel-order`
- `POST /api/v2/mix/order/cancel-batch-orders`
- `POST /api/v2/mix/order/cancel-all-orders`
- `POST /api/v2/mix/order/modify-order`
- `POST /api/v2/mix/order/close-positions`
- `GET /api/v2/mix/order/detail`
- `GET /api/v2/mix/order/orders-pending`
- `GET /api/v2/mix/order/orders-history`
- `GET /api/v2/mix/order/fills`

### Spot Wallet and Asset

- `GET /api/v2/spot/public/coins`
- `GET /api/v2/spot/wallet/deposit-address`
- `GET /api/v2/spot/wallet/deposit-records`
- `GET /api/v2/spot/wallet/withdrawal-records`
- `GET /api/v2/spot/wallet/transfer-coin-info`
- `POST /api/v2/spot/wallet/transfer`
- `POST /api/v2/spot/wallet/withdrawal`

## Bitget WebSocket Support

- `bitget_rs::api::websocket::BitgetWebsocket` 使用 Bitget V2 public/private WebSocket 主域名，默认分别为 `wss://ws.bitget.com/v2/ws/public` 和 `wss://ws.bitget.com/v2/ws/private`，可通过 `BITGET_WS_PUBLIC_URL` / `BITGET_WS_PRIVATE_URL` 覆盖。
- WebSocket login 按官方规则签名：`timestamp + GET + /user/verify`，HMAC-SHA256 后 base64，payload 为 `{"op":"login","args":[...]}`。
- 订阅与取消订阅统一使用 `BitgetWebsocketChannel { instType, channel, instId, coin }`，输出 `{"op":"subscribe","args":[...]}` / `{"op":"unsubscribe","args":[...]}`。
- WebSocket trade operation helper 支持构造 `place-order` / `cancel-order` 的 `{"op":"trade","args":[...]}` payload：`BitgetWebsocketPlaceOrderParams` 覆盖 futures 下单核心字段，`BitgetWebsocketCancelOrderParams` 覆盖 `orderId` / `clientOid` 二选一取消语义。
- `BitgetWebsocketSession` 支持 public/private URL 连接、SOCKS5/SOCKS5h 代理、字符串 `ping`、字符串 `pong` 解析、登录消息发送、订阅/取消订阅和 typed event 接收。
- `BitgetWebsocketEvent` 当前区分 `Pong`、login ack、subscribe ack、unsubscribe ack、trade ack、error、ticker、orders、account、positions、orderbook、trades、candles、fill、generic data push 和 raw fallback；typed data push 保留 `action`、`arg`、typed `data` 和 `raw`。
- 当前 typed DTO 覆盖 Bitget 官方常用推送字段：`BitgetTickerUpdate`、`BitgetOrderUpdate`、`BitgetAccountUpdate`、`BitgetPositionUpdate`、`BitgetOrderBookUpdate`、`BitgetTradeUpdate`、`BitgetCandleUpdate`、`BitgetFillUpdate`。未知频道仍返回 generic `Data`，避免因新增频道破坏兼容性。
- `BitgetWebsocketManager` 提供基础稳定性能力：连接状态、健康指标、自动重连、重连后订阅重放和定时 ping。默认 ping 间隔 30 秒，对齐 Bitget 官方保持连接建议；如果连续 3 个 ping 周期没有收到任何 text/binary/ping/pong 入站消息，会记录 `last_error` 并主动重连，覆盖 TCP 半开或服务端不再响应 `pong` 的场景。
- `BitgetWebsocketManager::with_login_credentials` 用于 private WebSocket：每次建立连接都会用当前时间重新生成 login payload，并等待 login ack 成功后再订阅，避免断线恢复后私有频道只重放订阅、不重新认证，或认证尚未完成就抢发订阅。
- `BitgetWebsocketManager::subscribe` / `unsubscribe` 支持连接运行中动态调整订阅：命令会发送到当前 socket，同时更新 manager 和重连循环里的订阅集合；取消订阅后的频道不会在下一次重连时被重放。
