# MIG-EXSDK-K1 Binance USD-M Public Kline Evidence

## 1. 身份与当前状态

| 字段 | 值 |
| --- | --- |
| Program / Owner child | `MP-market-data-foundation-v1 / MIG-EXSDK-K1-BINANCE-USDM-PUBLIC-KLINE-V1 / Exchange SDK` |
| Owner repository | `crypto_exc_all` |
| Registration revision | `rust_quant@f4acef65caca988b8ee9cd5ef9f1f4dd9d3e1c82` |
| SDK base revision | `crypto_exc_all@db467416bb4c3d5f895e7f16a61b32768e79a61b` |
| Repository profile | `rust_quant_alpha@92401760f9eed593c17bc30c4ce6ef0a1d4d9684` |
| 技术状态 | `implementing` |
| 模式 | `behavior_change` |
| Cutover | `not_required` |
| Production mutation | 禁止，未执行 |
| Machine Verdict | 不创建；前置 I1 尚无 current-revision Verdict |

## 2. 四层认知

### 已知的已知

- legacy SDK 已有正确匿名 endpoint 和 request 参数，但返回 raw `Value`；
- root legacy mapper 会对坏 OHLC/volume 填空字符串，并丢失 trade/taker-buy 字段；
- Core 增量同步、REST finality 推断、WebSocket `k.x`、分表写入和 research 月包是不同责任；
- K1 只覆盖 Binance USD-M，V1 不扩展其他 provider。

### 已知的未知

- 当前官方生成文档展示按 `limit` 分段计费，但 request schema 未稳定声明最大值；
- provider 后续可能在标准 12 个位置后增加字段；
- REST 当前棒何时可视为 final 需要 Market observation-time contract，不是 DTO 能决定。

### 未知的已知

- “旧接口已经能返回 K 线”不等于坏行、精度、限频证据和 caller ownership 已闭合；
- Core 使用 dummy credential 构造 full gateway 只是 legacy 装配债务，不代表公共 K 线需要凭证；
- `close_time <= now` 依赖本机时钟且不能等价替代 WebSocket `k.x`；
- research Binance Vision 的 checksum/连续性证据不能由 REST endpoint fixture 替代。

### 未知的未知

- provider 可能混合 string、JSON number、科学计数法或追加未知列；
- NAT、代理、地区域名和 provider 规则可能改变真实 quota header；
- 空数组、短页、重复 open time 或跨页重叠需要 Market 恢复协议，SDK不能自行猜测。

## 3. 官方协议观察

- 2026-07-30 核验的
  [Binance 官方生成 REST 文档](https://binance.github.io/binance-connector-js/classes/_binance_derivatives-trading-usds-futures.DerivativesTradingUsdsFuturesRestAPI.RestAPI.html)
  与
  [固定 revision 的生成源码](https://raw.githubusercontent.com/binance/binance-connector-js/a9de9a99bb4e1bd796e1ccf96088514064cc0b7e/clients/derivatives-trading-usds-futures/src/rest-api/modules/market-data-api.ts)
  把 K 线请求映射为匿名 `GET /fapi/v1/klines`，参数为
  `symbol/interval/startTime/endTime/limit`；
- 当前生成文档说明 K 线按 open time 唯一，并按 `limit` 分段计算 request weight；
- 当前
  [request schema](https://binance.github.io/binance-connector-js/interfaces/_binance_derivatives-trading-usds-futures.DerivativesTradingUsdsFuturesRestAPI.KlineCandlestickDataRequest.html)
  没有给出可稳定冻结的最大 `limit`，而
  [旧版官方 connector](https://github.com/binance/binance-futures-connector-python/blob/main/binance/um_futures/market.py)
  曾写 default 500 / max 1000。为避免把不一致文档固化为 SDK 行为，K1 只拒绝
  `limit=0`，其余值原样交给 provider；Market F2B 再冻结 operational page cap；
- [Binance public data 文档](https://github.com/binance/binance-public-data/blob/master/README.md?plain=1)
  给出 12 个 USD-M K 线列，并提示归档数据可能后续修正。

官方文档会演进，上述内容是本次观察输入；fixture 证明冻结 contract，不替代 future
live read-only probe 或 Market completeness。

## 4. legacy 语义和调用点

- 业务语义逐项处置见
  [business-semantics-matrix.md](business-semantics-matrix.md)；
- committed-object 搜索命令、计数和 caller 分类见
  [callsite-closure.md](callsite-closure.md)；
- K1 不修改 `src/adapters/binance.rs`、Core、数据库、WebSocket 或 research 文件。

## 5. 实施放置声明

- Owner：Exchange SDK；
- capability：`public_market/binance_usdm_kline`；
- slice：provider-specific read-only protocol adapter/facade；
- entry：`BinanceUsdmPublicKlineClient::klines`；
- Domain / Use Case / Port：不创建；
- model：`BinanceUsdmKline` wire DTO；
- adapter：Binance anonymous `GET /fapi/v1/klines`；
- public surface：root `binance-public-kline` 最小 feature、typed response/failure；
- transaction / persistence / outbox：无；
- recovery owner：Market；
- runtime / binary / Release Unit：无。

## 6. 当前工作树边界

开始 K1 前仅存在用户的 `README.md` 修改；它不在 K1 allowlist，不会被覆盖、暂存、
回退或纳入精确 patch。主工作树 migration-check 应继续对该 forbidden path fail-closed；
K1 自身需要用精确 patch/临时干净 checkout 单独验证。

## 7. TDD 与验证结果

| 验证 | 当前结果 | 备注 |
| --- | --- | --- |
| Manifest schema / authority / pinned artifacts | PASS | migration-check 对应阶段均为 `errors=0 warnings=0` |
| RED | PASS | bnb test 因缺 `get_klines_typed/InvalidRequest` 失败；root test 因 feature 不存在失败 |
| Binance typed Kline contract | PASS | `kline_api_tests` 6/6 |
| Root minimum feature facade | PASS | `binance_public_market_tests` 2/2 |
| Legacy raw compatibility | PASS | `expanded_api_tests::market_wrappers_map_core_public_endpoints` 1/1 |
| Existing OKX public-market regression | PASS | `okx_public_market_tests` 2/2 |
| Focused Clippy | PASS | bnb/root 两条 `-D warnings` 命令通过 |
| Feature compatibility | PASS | 最小 Kline 与 legacy `binance` 均编译；最小 cfg 有 Kline、无 `full-sdk` |
| Format / diff / source guards | PASS | format、K1 scope diff-check、无业务 `f64`、无认证/签名命中 |
| File budget | PASS（baseline warning） | 本轮文件均低于 1000 行；脚本只报告既有 OKX/Binance adapter 1714/1554 行 warning |
| Migration declared scope / Registry identity / DAG | PASS | required artifacts、target closure、registration identity 与 dependency gates 均为 0 错误 |
| Clean profile-first migration-check | PASS | 使用正式 profile commit 的 disposable clean clone，只应用 K1 allowlist patch：`errors=0 warnings=0` |
| Immutable repository profile | PASS | `rust_quant_alpha@92401760...` 已登记 `binance-public-kline`，profile 两阶段均为 0 错误 |
| Main worktree diff scope | BLOCKED（pre-existing） | 用户原有 `README.md` 命中 forbidden path |
| Production/live mutation | 未执行 | 本切片不允许 |

测试冻结在 SDK base
`db467416bb4c3d5f895e7f16a61b32768e79a61b` 加 source/test patch SHA-256
`d41792c946a69208c319d00b8952aa2a7ddd066c4a08a489f69ca44cfcfab83c`。
该 patch 为 39,914 bytes，使用独立临时 Git index 仅收录 Manifest allowlist 中的
12 个 source/test 路径，明确排除 `README.md`、自引用文档与 `task_plan.md`。
13 个 source/test/task-plan output artifact 的逐文件 SHA-256 已写入 Manifest。

正式 profile 已作为独立提交
`rust_quant_alpha@92401760f9eed593c17bc30c4ce6ef0a1d4d9684` 固化，内容
SHA-256 为
`37b448c591c3c0096fe59e233ec0e171bbc14d5c41ecea51943cf885e0213d91`。
当前主工作树 migration-check 的 profile drift 与 unknown feature 已归零；唯一错误是
K1 开始前已存在、且明确不属于本迁移的 `README.md` forbidden diff。

另用正式 profile commit、Registry registration commit 与 SDK base commit 建立
disposable clean clones，只应用 K1 Manifest allowlist patch。该组合的 P2
migration-check 为 `errors=0 warnings=0`；临时验证目录不作为迁移输入或产物。

## 8. 未完成项与提交门

- typed DTO、provider API、root facade、contract tests、output hash 与 exact patch hash
  已落入工作树；
- `rust_quant_alpha/architecture/repository-profiles/crypto_exc_all.toml` 已单独提交，
  K1 Manifest 已固定真实 immutable revision/hash，clean exact-patch 检查通过；
- 前置 I1 没有 current-revision Verdict，K1 不得进入 `verified/completed`；
- 下一步提交 K1 工件，并用已提交 Manifest/Evidence hash 将 Registry 从
  `not_created` 更新为 `created`；
- K1 Registry `created` 前，不允许建立或消费 Market F2B Manifest。
