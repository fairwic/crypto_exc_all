# I1 旧业务语义与调用点处置矩阵

## 1. 目的与结论

本文件先回答“旧行为是否有业务价值、由谁承接”，再允许写 I1 源码。决策只有四类：

- `保留`：业务意图和当前语义都必须继续存在；
- `优化`：保留业务目的，但替换不安全、无类型或跨层实现；
- `废弃`：错误事实或错误协议入口不得进入目标链路；删除仍需独立门禁；
- `延期`：属于完整闭环，但 Owner 不是 Exchange SDK 或不在 I1。

关键结论：

1. I1 只实现 Binance USDⓈ-M 与 OKX SWAP 的 provider wire capability；
2. SDK 不选择币种池、不判断 snapshot 业务完整性、不映射 canonical status/Decimal，也不拥有重试、调度、数据库或上市信号；
3. 既有 Binance typed 缺口与 OKX path/auth 冲突必须优化；既有研究、执行 smoke、持久化和 App 入口只形成字段与错误语义约束，不在本轮迁移；
4. 其他 provider 不禁用、不删除，也不计入 V1 验收。

冻结 revision：

- SDK：`crypto_exc_all@c17ba15185a337e03df5dfe4ecf08e7fd3e8a380`；
- legacy Core committed baseline：
  `rust_quant@30789257dfea817cbde91d38fe91fd60f638c478`；
- Architecture Governance/profile：
  `rust_quant_alpha@9b755b9fa2e24bc5c0a103a836978df1504c070e`；
- F4B SDK successor 责任校正：
  `rust_quant_alpha@b36731bed29213739d3c08541c9e0cca6b876d35`。

## 2. SDK 语义

| ID | 冻结位置与当前行为 | 决策 | I1 目标语义 | 首个差异与验证 |
| --- | --- | --- | --- | --- |
| `EXSDK-LEG-001` | `binance_rs/src/api/market/market_api.rs`：`new_public()` 正确调用无 query 的 `/fapi/v1/exchangeInfo`，但只返回 `serde_json::Value` | 优化 | 保留 legacy Value 方法；新增 USDⓈ-M typed response、provider DTO 与 root public-only client | 首差在 response decode；mock 断言 exact path/query、无 API key、`contractType`/filters round-trip |
| `EXSDK-LEG-002` | `binance_rs/src/client.rs`：成功与错误都在读 body 前丢弃 headers；只保留 status/code/msg | 优化 | 新 public response/failure 保留 HTTP status；`x-mbx-used-weight-*`、order-count 与 `Retry-After` 仅在响应实际携带时原样保留；不内置重试 | 429/418/provider error fixture 分别证明 present header 可见、missing header 保持 absent |
| `EXSDK-LEG-003` | `okx_rs/src/api/public_data/public_data_api.rs`：正确 `/api/v5/public/instruments` 却调用 signed transport，`from_env` 要求账户 credential | 优化，P0 | 新 public-data client 使用 `OkxClient::new_public`，固定 `instType=SWAP`，不携带签名/passphrase/模拟交易头 | 首差在 auth transport；mock 断言 exact query 和所有认证头缺席 |
| `EXSDK-LEG-004` | `okx_rs/src/api/market/market_api.rs`：anonymous transport 调用错误 `/api/v5/market/instruments`；冻结代码无生产调用方 | 废弃入口、暂不删除 | I1 facade 永不调用该入口；未来有版本和外部调用证据后再删除 | 全仓新 facade path guard 只允许 `/api/v5/public/instruments` |
| `EXSDK-LEG-005` | `InstrumentOkxResDto` 只含 instType/instId/uly/base/quote/tick/lot/min/state | 优化 | 新 public-data DTO 保留 family/category/settle/contract value/type/listing/expiry/max-size/rule 以及未知扩展 | 完整 fixture、未知字段/status 与 decimal wire round-trip |
| `EXSDK-LEG-006` | `okx_rs/src/client.rs`：公共 transport 可用，但 429/5xx 退化为普通 `OkxApiError` 且 headers 丢失；`RateLimitError` 未与 HTTP 建立 | 优化 | new public response/failure 保留 HTTP status、OKX code/msg；安全 quota headers 与 `Retry-After` 仅在响应实际携带时保留；不做 sleep/retry | HTTP 200/code!=0、429、5xx、malformed success 以及 header present/absent contract test |
| `EXSDK-LEG-007` | root `public_market` 只有 OKX K 线；`CryptoSdk` 需要 credential；`Instrument` 是调用方 canonical symbol，不是 provider metadata | 保留并优化 | 原 K 线能力和 canonical `Instrument` 不变；新增两个具体 public-instrument client 和独立 `binance-public-instrument` feature，不造 provider trait、不暴露 mutation 方法 | root facade test 通过最小 feature 获得 typed provider response；编译 cfg 证明未启用 root `full-sdk`，旧 `binance` feature 继续兼容 |
| `EXSDK-LEG-008` | `binance_rs/examples/live_post_only_order.rs` 依赖 Value，并用 f64 构造 live post-only plan；现有 test 只验 path/timezone | 保留 example、优化测试、延期 caller | I1 不修改或运行 live example；新增 deterministic mock contract test | 新测试覆盖字段、精度、unknown、auth、error/quota；example 不作完成证据 |

## 3. legacy Core/App/Research 语义

| ID | 冻结位置与当前行为 | 决策 | 后继 Owner 与必须承接语义 | I1 约束 |
| --- | --- | --- | --- | --- |
| `RQ-LEG-001` | `ExchangeSymbolSyncService`：Binance 用 raw SDK；OKX 用 20s direct reqwest；source 失败上抛，六家串行 | 延期 | Market F4C 通过 I1 获取两家 typed response，拥有 source profile、失败汇总和持续 ingest | I1 只提供 wire response；Bitget/Bybit/Gate/KuCoin 不删除 |
| `RQ-LEG-002` | 同 service：Binance 只选 PERPETUAL，形成 canonical `BASE-QUOTE-SWAP`；OKX 只选 SWAP，并提取 precision/rules/status/raw payload | 延期并优化 | Market F4C 冻结 perpetual/quote/settle 选择与 canonical lifecycle；独立 InstrumentRules successor 拥有 Decimal rules、有效时间和历史查询 | I1 必须无损提供 selection/rules 所需 wire 字段，不执行筛选和领域映射 |
| `RQ-LEG-003` | sync 先逐行 first-seen insert 再 current upsert，随后可发 major-listing signal/icon；非单事务 | 保留业务证据、延期/废弃跨层副作用 | Market F4C 承接 lease/cursor/run ledger；Strategy/Web 各自承接信号与图标，metadata ingest 不直接下单 | I1 无 DB、Signal、ExecutionRequest、Outbox |
| `RQ-LEG-004` | CLI/worker/manual HTTP 共用 sync；run ID 含 wall-clock；source request/env/code 默认互相不同；worker 周期失败后等下一轮 | 延期 | Market F4C 统一 typed source profile、唯一 lease、cursor、bounded retry 和 run ledger | SDK 不读 runtime inventory/env，不拥有 scheduler/retry |
| `RQ-LEG-005` | all-market monitor 启动时 direct OKX SWAP，和 DB active 集合求交；空集合 fail closed | 延期 | Market readiness/caller migration 保留一次性 warmup、stale audit、empty blocker | I1 空 data 只作为 typed response，不替 Market 判定 outage |
| `RQ-LEG-006` | OKX historical universe 读取 instFamily/instCategory/settle/state/ctVal 等，筛 live/category1/USDT；ctVal 转 f64；4 次线性等待 | 延期并优化 | Research/Market successor 保留筛选与证据但改用 point-in-time Dataset；retry 由 App 管理 | I1 保留 decimal String/Number，禁止 f64、筛选和 survivorship policy |
| `RQ-LEG-007` | strict-static Top60 对完整 OKX envelope 和单 instrument 做 hash，再筛 live/linear/USDT/listTime/tick；不足 fail | 延期 | Research 继续拥有 hash/version/universe 算法和 no-look-ahead | I1 DTO 不能被当作既有 raw hash 的等价序列化格式 |
| `RQ-LEG-008` | cross-exchange research 共享 Binance helper，4 次重试；筛 TRADING/PERPETUAL/USDT/COIN，含 1000* 与 LUNA2 映射 | 延期 | Research caller/F4C 迁移保留确定性筛选、映射和失败证据 | I1 typed DTO 必须保留 status/contract/quote/underlying，但不含业务 mapping |
| `RQ-LEG-009` | flow-flip 与 orderbook panel 各自 direct Binance exchangeInfo，分别 30s/60s 单次；重复同一资格过滤 | 延期 | 后续 caller 复用 I1/F4C；研究并发和 timeout 仍由 Research/App 管理 | I1 不趁机重写研究流程 |
| `RQ-LEG-010` | Binance ETH micro live validation 用持有用户 key/secret 的 client 读取 public exchangeInfo，解析 tick/step/minNotional 后进入 signed preflight/可能 mutation | 延期并最终废弃 smoke | Execution successor 先完成 public filters、signed preflight、止损和 live 门禁 parity，再移除旧 smoke | I1 只提供纯 public filter wire；绝不运行或装配 mutation |
| `RQ-LEG-011` | execution、scanner、backfill、handoff、event backtest 与 Vegas 继续从 `exchange_symbols/raw_payload` 读 status、rules、listTime、ctVal 等 | 保留需求、延期 | Market F4C/current readiness/historical InstrumentRules 和各 consumer successor | 字段需求证明 DTO 不得裁剪，但 SDK 不拥有 DB schema/consumer |

## 4. 搜索闭包

完整、可复制的命令、精确 pathspec、逐文件行号与每个命中到矩阵 ID 的 ledger 见
[callsite-closure.md](callsite-closure.md)。摘要如下：

| 冻结搜索 | 命中行 |
| --- | ---: |
| SDK instrument endpoint/caller | 11 |
| Core Binance acquisition | 19 |
| Core OKX acquisition | 10 |
| Core 同步运行入口 | 15 |
| Core `exchange_symbols` 生产消费者 | 36 |
| root feature 事实 | 4 |
| SDK auth/public 边界 | 61 |
| legacy instrument header capture | 0 |
| SDK HTTP status/error | 9 |
| I1 相关 legacy `f64` | 8 |

所有搜索只针对冻结 commit，不读取当前脏工作树推断调用关系；测试、example、注释不冒充
生产 caller。

## 5. Owner 边界与首个差异层

```text
provider HTTP response
  -> SDK HTTP status/quota evidence
  -> provider envelope/error
  -> provider-specific typed DTO              I1 到此为止
  -> Market source profile/completeness
  -> canonical lifecycle                      Market F4C
  -> Decimal InstrumentRules                  独立 Market successor
  -> scheduler/lease/cursor/run ledger
  -> persistence/readiness/consumer
```

首个差异层是 provider response decode，不是 Market selection：

- Binance 从 raw `Value` 增加 typed response，同时保留 legacy 方法；
- OKX 从“正确 path + 错误 signed transport”和“错误 path + anonymous transport”收敛到
  唯一正确的 anonymous public-data capability；
- 数字在 SDK 保持 wire String/任意精度 Number；规则值第一次转为 Decimal 属于独立
  InstrumentRules successor，不进入 F4C lifecycle Aggregate。

## 6. 删除门与未完成项

- I1 不删除任何 legacy caller、数据库列、example、其他 provider module 或 runtime 配置；
- OKX 错误 path wrapper只有在外部调用/版本兼容证据关闭后才能删除；本轮只保证新 facade
  不使用；
- Binance legacy Value 方法等 Market F4C 与剩余 caller 完成 parity 后再单独移除；
- root 现有 `binance` feature 继续包含 legacy `full-sdk`，保持既有调用方语义；
  Architecture Governance 已在 `rust_quant_alpha@9b755b9fa2e24bc5c0a103a836978df1504c070e`
  更新 repository profile，I1 新增 `binance-public-instrument` 最小 feature。Market
  public gateway 只启用该 feature 与 `okx-public-market`，不能获得 root `CryptoSdk`、
  `raw`、账户或交易门面；
- `README.md` 是开始 I1 前已经存在的用户改动，不属于矩阵、allowlist 或提交范围。

Market F4C 未引用本矩阵并逐项承接 `RQ-LEG-*` 前，不得宣称 instrument metadata 业务链路
已经迁移或可切换生产。
