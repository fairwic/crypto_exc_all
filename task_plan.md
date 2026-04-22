# Task Plan: Exchange SDK Aggregation

## Goal

在已完成 OKX/Binance 聚合与发布经验的基础上，继续把 Bitget 接入为独立 SDK crate，并在根 crate `crypto_exc_all` 中提供统一聚合入口。

## Phases

- [x] Phase 1: Plan and setup
- [x] Phase 2: Research project architecture and unfinished work
- [x] Phase 3: Research Binance official API/SDK guidance
- [x] Phase 4: Write architecture and implementation plan
- [x] Phase 5: Implement one prioritized unfinished Binance feature with tests
- [x] Phase 6: Verify and deliver
- [x] Phase 7: Add Bitget independent SDK crate and root aggregation
- [x] Phase 8: Verify Bitget aggregation and update integration playbook
- [x] Phase 9: Add root unified positions and basic trade facade for OKX/Binance/Bitget
- [x] Phase 10: Add root unified order query facade for OKX/Binance/Bitget
- [x] Phase 11: Add root unified fills facade for OKX/Binance/Bitget
- [x] Phase 12: Add root unified orderbook and candles facade for OKX/Binance/Bitget
- [x] Phase 13: Add root unified derivatives market metrics for OKX/Binance/Bitget
- [x] Phase 14: Add root unified market sentiment stats for OKX/Binance/Bitget
- [x] Phase 15: Add root unified set-leverage account setting for OKX/Binance/Bitget
- [x] Phase 16: Add root unified set-position-mode account setting for OKX/Binance/Bitget
- [x] Phase 17: Add Bitget V2 WebSocket base client, events, and reconnect manager
- [x] Phase 18: Add Bitget WebSocket typed DTOs for ticker/orders/account/positions
- [x] Phase 19: Add Bitget WebSocket typed DTOs for books/trade/candles
- [x] Phase 20: Add Bitget WebSocket typed DTO for private fill channel
- [x] Phase 21: Add Bitget WebSocket place/cancel trade request helpers and ack parser
- [x] Phase 22: Add Bitget WebSocket inbound stall reconnect guard
- [x] Phase 23: Add Bitget WebSocket private login replay and ack gate before subscription replay
- [x] Phase 24: Add Bitget WebSocket runtime subscribe/unsubscribe command path
- [x] Phase 25: Count connected-session failures against Bitget WebSocket max reconnect attempts

## Key Questions

1. 项目当前的模块边界、配置加载、交易所抽象和执行入口是什么？
2. README 或代码中标记的未完成功能有哪些，哪个最适合作为第一项交付？
3. Binance USDⓈ-M Futures 官方当前推荐的 SDK 或 REST/WebSocket 接入方式是什么？
4. 如何在不泄露 `.env` 密钥的前提下复用现有配置完成可验证实现？

## Decisions Made

- 使用持久化计划文件跟踪该多阶段任务。
- Binance 第一阶段采用与 OKX 对齐的本地轻量 SDK 结构，而不是直接暴露官方自动生成 `binance-sdk` 类型。
- 优先交付 Binance USDⓈ-M Futures 只读签名接口 `GET /fapi/v2/balance`，同时实现公共 `GET /fapi/v1/time`。
- 第二阶段继续按 OKX 主要能力补 Binance REST wrapper：市场数据、账户/持仓、订单/成交记录。
- 用户明确要求执行真实下单测试后，新增受控实盘 post-only example：按实时交易规则构造 `LIMIT + GTX` 最小挂单，接受后立即撤单。
- 第三阶段继续补 OKX 对齐缺口，优先补 Binance Futures 核心 REST：批量订单、改单、全部撤单、账户配置、交易限制、杠杆档位、持仓模式、mark price/open interest/多空比/taker volume。
- 第四阶段继续补 OKX 对齐缺口中的资产域：采用 Binance Wallet/SAPI REST，新增独立 `BINANCE_SAPI_API_URL`，但复用现有签名、错误解析和 builder 风格。
- 第五阶段补公告和 WebSocket 必要 API 接口：公告采用 Binance 网站 BAPI best-effort wrapper，WebSocket 补 listenKey REST、2026-04-23 前需迁移的新分流 URL 构造器、JSON 消息循环、订阅/取消订阅、基础自动重连订阅重放和 SOCKS5/SOCKS5h 代理连接。
- 第六阶段补 WebSocket 稳定性增强：按 Binance 官方 `/public`、`/market`、`/private` 分流多连接，增加连接状态/健康度指标，为核心私有事件 `ORDER_TRADE_UPDATE`、`ACCOUNT_UPDATE` 提供 typed parser，并区分 combined stream URL 与 JSON `SUBSCRIBE` 模式，避免重复订阅。
- 第七阶段继续补 WebSocket typed parser 完整性：覆盖 `listenKeyExpired`、`MARGIN_CALL`、`TRADE_LITE`、`ACCOUNT_CONFIG_UPDATE`、`STRATEGY_UPDATE`、`GRID_UPDATE`、`CONDITIONAL_ORDER_TRIGGER_REJECT`、`ALGO_UPDATE`，并支持 combined stream `{stream,data}` 私有事件解包。
- Bitget 接入采用和 Binance 一样的双层结构：`bitget_rs` 只负责 Bitget V2 原生 API/签名，根 crate adapter 负责统一 `ExchangeId`、`Instrument`、`Ticker`、`Balance` 和错误映射。
- Bitget 第一阶段以 USDT 永续只读能力为聚合面：`GET /api/v2/mix/market/ticker` 和签名账户列表 `GET /api/v2/mix/account/accounts`，默认 `productType=USDT-FUTURES`，后续再扩展交易、WebSocket 和更多 product type。
- Bitget 第二阶段补齐 OKX/Binance 已有 REST 域的主要等价能力：market/account/trade/asset/announcements/common trade-rate。根 facade 继续只稳定暴露跨交易所 `ticker` 和 `balances`，交易所特有细节通过 `crypto_exc_all::raw::bitget` 使用。
- 根 crate 第三阶段在已有 `ticker` / `balances` 基础上补统一 `positions`、`place_order`、`cancel_order`。统一 DTO 只覆盖三家可稳定对齐的字段；订单高级参数、批量单、改单、资产划转等仍先走 `raw` 入口。
- 根 crate 第四阶段补统一订单查询 facade：`OrderFacade::get/open/history` 对齐 Binance `order/openOrders/allOrders`、OKX `order/orders-pending/orders-history`、Bitget `detail/orders-pending/orders-history`，统一输出 `Order` DTO，保留 `raw` 字段承载交易所差异。
- 根 crate 第五阶段补统一成交明细 facade：`FillFacade::list` 对齐 Binance `userTrades`、OKX `fills`、Bitget `fills`，统一输出 `Fill` DTO，覆盖成交 ID、订单 ID、方向、价格、数量、手续费、手续费币种、maker/taker、时间戳和 raw。
- 根 crate 第六阶段扩展 `MarketFacade`：`orderbook` 对齐 Binance `depth`、OKX `books`、Bitget `orderbook`；`candles` 对齐 Binance `klines`、OKX `candles`、Bitget `candles`，统一输出 `OrderBook` 和 `Candle` DTO。
- 根 crate 第七阶段扩展 `MarketFacade` 衍生品市场指标：`funding_rate`、`funding_rate_history`、`mark_price`、`open_interest` 对齐 Binance、OKX、Bitget 的公共 REST，统一输出 `FundingRate`、`MarkPrice`、`OpenInterest` DTO，并为 OKX SDK 补齐 `public/mark-price` 与 `public/open-interest` wrapper。
- 根 crate 第八阶段扩展 `MarketFacade` 市场情绪统计：`long_short_ratio` 和 `taker_buy_sell_volume` 对齐 Binance futures data、OKX rubik big-data、Bitget mix market 统计接口，统一输出 `LongShortRatio` 和 `TakerBuySellVolume` DTO。
- 根 crate 第九阶段扩展 `AccountFacade` 交易设置：`set_leverage` 对齐 Binance `POST /fapi/v1/leverage`、OKX `/api/v5/account/set-leverage`、Bitget `/api/v2/mix/account/set-leverage`，统一输出 `LeverageSetting` DTO。OKX 合约类 instrument 只发送 `instId/lever/mgnMode/posSide`，避免把 Bitget 所需的 `marginCoin` 误映射为 OKX `ccy`。
- 根 crate 第十阶段扩展 `AccountFacade` 持仓模式设置：`set_position_mode` 对齐 Binance `POST /fapi/v1/positionSide/dual`、OKX `/api/v5/account/set-position-mode`、Bitget `/api/v2/mix/account/set-position-mode`，统一 `PositionMode::OneWay/Hedge`，并为 OKX SDK 补齐原生 `set_position_mode` wrapper。
- 根 crate 第十一阶段扩展账户能力发现和 symbol 保证金模式设置：`AccountFacade::capabilities()` 暴露 `set_symbol_margin_mode` 与 `order_level_margin_mode`；`set_symbol_margin_mode` 对齐 Binance `POST /fapi/v1/marginType` 和 Bitget `/api/v2/mix/account/set-margin-mode`，OKX 明确返回 `Unsupported`，避免把 OKX 下单 `tdMode`/杠杆 `mgnMode` 伪装成持久 symbol 配置。
- 根 crate 第十二阶段扩展策略层保证金模式入口：`ensure_order_margin_mode` 对 Binance/Bitget 复用 symbol 保证金模式真实切换，对 OKX 返回 order-level 应用结果，不发送不存在的配置请求；统一输出 `MarginModeApplyMethod::SymbolConfiguration/OrderLevel`。
- 根 crate 第十三阶段扩展策略层下单前预配置入口：`prepare_order_settings` 聚合 position mode、margin mode、leverage 三类可选配置，按持仓模式、保证金模式、杠杆顺序执行，并复用已有三家 adapter 能力，避免策略层散落交易所分支。

## Errors Encountered

- Live Binance read-only request failed before reaching Binance because local `all_proxy` uses `socks5://127.0.0.1:7897` and `reqwest` did not enable the `socks` feature. Resolution: enable `reqwest/socks` in `binance_rs/Cargo.toml`.
- After enabling SOCKS support, signed live request and public Binance time endpoints still timed out through the default current network/proxy path. Resolution: explicitly normalize `socks5://` proxy URLs to `socks5h://` and wire proxy configuration into `reqwest::Client`.
- Real `POST /fapi/v1/order` was attempted against Binance USDⓈ-M Futures mainnet through the current proxy. Binance rejected it before order creation with code `-2015` (`Invalid API-key, IP, or permissions for action`), so no `orderId` was created and no cancel request could be issued.
- After the IP whitelist update, Binance accepted a real DOGEUSDT `LIMIT + GTX` post-only order and the example immediately canceled it. Earlier BTCUSDT attempts surfaced Hedge Mode `positionSide` and insufficient-margin constraints.
- Bitget live read-only account example initially failed locally before request dispatch because `.env` used `bitget_PASSPHRASE` while the SDK accepted only `BITGET_PASSPHRASE` / lowercase variants. Resolution: add this existing mixed-case key name to both `bitget_rs::Credentials` and root `BitgetExchangeConfig` env lookup, with tests.
- After the env-name fix, Bitget live read-only V2 account request reached the exchange but Bitget returned code `40012` (`apikey/password is incorrect`). A separate `curl + openssl` reproduction returned the same `40012` for multiple V2 signed endpoints, while the same key/passphrase authenticated against UTA V3 and returned `40084` (`You are in Classic Account mode, and the Unified Account API is not supported at this time`). This points to a Bitget key/account-mode compatibility issue for V2 signed APIs, not a local signing/query/body implementation failure.
- After replacing the Bitget API key/passphrase and adding the current egress IP to the whitelist, Bitget V2 signed account reads succeeded. A real `post_only` BTCUSDT futures order attempt reached the trade endpoint but was rejected with `40762` (`The order amount exceeds the balance`) because USDT-Futures available balance and spot USDT available balance are both `0`.

## Status

**Currently complete for Bitget REST aggregation parity plus root market/account-setting/trade/position/order-query/fills/derivatives-metrics/sentiment-stats facade and Bitget WebSocket base client** - Binance market/account/trade REST wrappers, the live post-only order harness, Wallet/SAPI asset wrappers, announcement wrapper, and WebSocket listenKey/URL/session/reconnect/split-route/health helpers are implemented and verified locally. Private user-data typed parsers now cover the current USDⓈ-M Futures event set documented by Binance. A real Binance Futures order was created on DOGEUSDT and immediately canceled with zero executed quantity. Bitget now has an independent `bitget_rs` crate plus root facade adapter for market ticker, orderbook, candles, funding rate, funding rate history, mark price, open interest, long-short ratio, taker buy-sell volume, account balances, set leverage, set position mode, set symbol margin mode with capability discovery, ensure order margin mode, prepare order settings, positions, basic order placement, cancellation, order detail, open orders, order history, and fills. `bitget_rs` now covers the major OKX/Binance REST domains: market data, account/position, order/trade, wallet/asset, announcements, and common trade-rate, and includes Bitget V2 WebSocket public/private URL config, login signing, ping/pong, subscribe/unsubscribe, place/cancel trade request helpers, trade ack parsing, typed base event parsing, ticker/orders/account/positions/books/trade/candles/fill typed DTOs, SOCKS5/SOCKS5h connection support, reconnect metrics, runtime subscribe/unsubscribe command path, timed ping, private login replay with login ack gate before subscription replay, inbound stall timeout reconnect, connected-session failure attempt limiting, and subscription replay.

Next remaining parity areas after Bitget REST/WebSocket base aggregation: optional root unified event stream facade, additional exchange-specific Bitget private channels such as ADL/trigger-order/history-position, Bybit, Hyperliquid, and exchange-specific public endpoints that do not have a direct USDⓈ-M Futures REST equivalent.
