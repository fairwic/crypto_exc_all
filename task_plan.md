# Task Plan: Binance Futures Feature Completion

## Goal

梳理当前项目架构，明确 Binance USDⓈ-M Futures 的最佳 SDK/API 接入方案，并优先完成一个未完成功能。

## Phases

- [x] Phase 1: Plan and setup
- [x] Phase 2: Research project architecture and unfinished work
- [x] Phase 3: Research Binance official API/SDK guidance
- [x] Phase 4: Write architecture and implementation plan
- [x] Phase 5: Implement one prioritized unfinished Binance feature with tests
- [x] Phase 6: Verify and deliver

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

## Errors Encountered

- Live Binance read-only request failed before reaching Binance because local `all_proxy` uses `socks5://127.0.0.1:7897` and `reqwest` did not enable the `socks` feature. Resolution: enable `reqwest/socks` in `binance_rs/Cargo.toml`.
- After enabling SOCKS support, signed live request and public Binance time endpoints still timed out through the default current network/proxy path. Resolution: explicitly normalize `socks5://` proxy URLs to `socks5h://` and wire proxy configuration into `reqwest::Client`.
- Real `POST /fapi/v1/order` was attempted against Binance USDⓈ-M Futures mainnet through the current proxy. Binance rejected it before order creation with code `-2015` (`Invalid API-key, IP, or permissions for action`), so no `orderId` was created and no cancel request could be issued.
- After the IP whitelist update, Binance accepted a real DOGEUSDT `LIMIT + GTX` post-only order and the example immediately canceled it. Earlier BTCUSDT attempts surfaced Hedge Mode `positionSide` and insufficient-margin constraints.

## Status

**Currently complete for REST/API parity areas implemented so far** - Binance market/account/trade REST wrappers, the live post-only order harness, Wallet/SAPI asset wrappers, announcement wrapper, and WebSocket listenKey/URL/session/reconnect/split-route/health helpers are implemented and verified locally. Private user-data typed parsers now cover the current USDⓈ-M Futures event set documented by Binance. A real Binance Futures order was created on DOGEUSDT and immediately canceled with zero executed quantity.

Next remaining parity areas: exchange-specific public endpoints that do not have a direct USDⓈ-M Futures REST equivalent.
