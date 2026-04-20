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
