# MIG-EXSDK-I1 Binance/OKX Public Instrument Evidence

## 1. 身份与当前状态

| 字段 | 值 |
| --- | --- |
| Program / Owner child | `MP-market-data-foundation-v1 / MIG-EXSDK-I1-BINANCE-OKX-PUBLIC-INSTRUMENT-V1 / Exchange SDK` |
| Owner repository | `crypto_exc_all` |
| Registration revision | `rust_quant@8acc7a42157fd7457ae72ddb2848240d9bbd5289` |
| SDK base revision | `crypto_exc_all@c17ba15185a337e03df5dfe4ecf08e7fd3e8a380` |
| P2/profile revision | `rust_quant_alpha@fe5e8e9d9c6b4462efda9ef00fd6bba64a9e73c3` |
| 技术状态 | `implementing` |
| 模式 | `behavior_change` |
| Cutover | `not_required` |
| Production mutation | 禁止，未执行 |
| Machine Verdict | 不创建；P2 predecessor 没有 current-revision Verdict |

P2 Manifest/Evidence 内容 hash 分别为
`4512ebac25b8afec6c73a755df4f833c8bc4151b7b7f29533e9c1aabf18699e8` /
`e951a6e569c56cc4ca961819f0629a929bcae3d65db7fa91289335a7f1699e27`；
SDK repository profile hash 为
`bffb9fcd50781ca91c16796d27fe6ec069d2eb9ea142cce08999c7edf945e975`。
P2 已形成 `crypto_exc_all` 实施输入，但其 `verdict.json` 不存在，因此 I1 不能进入
`verified/completed`。

## 2. 四层认知

### 已知的已知

- Binance 的 `/fapi/v1/exchangeInfo` path 与 anonymous client 已正确，但 response 是
  `serde_json::Value`；
- OKX 正确 public-data path 与 anonymous transport 分别存在于两个互相冲突的入口；
- 两个 endpoint 都是 public read-only；I1 不需要固定平台 key或用户交易 credential；
- V1 只覆盖 Binance USDⓈ-M 与 OKX SWAP。

### 已知的未知

- provider 会继续增加未知字段/enum/filter，typed DTO 必须前向保留；
- 真实 quota header 受出口 IP、地区和 provider 策略影响，SDK只能回传观测证据；
- root `binance` feature 当前仍编译 full-sdk，物理依赖闭包隔离需要独立治理后继。

### 未知的已知

- “endpoint 已存在”不等于 path/auth、required fields、错误和 quota contract 已闭合；
- Binance `pricePrecision/quantityPrecision` 不能替代 filter 中的 tick/step；
- OKX `instType=SWAP` 只定义协议 request scope，不等于 Market 已选择 USDT、linear 或 live；
- response 空集合与业务完整性、退市推断是不同语义。

### 未知的未知

- provider 可能返回 string/number/scientific decimal 混合、HTTP 200 provider error 或损坏
  success body；
- 新产品可能复用现有 endpoint，未知 status/contract/rule 不得由 SDK猜测；
- NAT、代理和地区域名可能改变 quota bucket 和实际返回 inventory。

## 3. 官方协议核验

- [Binance USDⓈ-M Exchange Information](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/market-data#exchange-information)
  在 2026-07-30 观察到 `GET /fapi/v1/exchangeInfo`，IP request weight 为 1；
- [OKX Public Data - Get instruments](https://www.okx.com/docs-v5/en/#public-data-rest-api-get-instruments)
  在 2026-07-30 观察到 public-data endpoint
  `GET /api/v5/public/instruments`；I1 固定 `instType=SWAP`。该页面当日展示的限额为
  20 requests / 2 seconds，rate limit rule 为 IP + Instrument Type；
- 上述数值是“文档观察值”，不是 SDK 内置重试/配额策略；quota 与
  `Retry-After` header 仅在 provider 实际响应携带时保留，文档没有出现的 header
  不得被 fixture 虚构为必然存在；
- decimal/rule wire 值按 string 或任意精度 JSON number 保留；I1 不转 `f64`。

官方文档会演进，fixture 只证明本 Manifest 冻结的 V1 contract，不能替代后续 live
public probe 或 Market readiness。

## 4. legacy 语义与搜索闭包

完整 19 项处置矩阵见
[business-semantics-matrix.md](business-semantics-matrix.md)，可复制命令、精确 pathspec
与逐命中 ledger 见 [callsite-closure.md](callsite-closure.md)。冻结 commit 摘要：

| 类别 | 命中 | 结论 |
| --- | ---: | --- |
| SDK instrument endpoint/caller | 11 | Binance raw、OKX 双入口、test/example 已全部归类；Bybit 只属延期 |
| committed Core Binance acquisition | 19 | 主同步、Research、Execution smoke 与 test 已归类 |
| committed Core OKX acquisition | 10 | 主同步、warmup、historical universe 已归类 |
| Core 同步入口 / `exchange_symbols` 消费者 | 15 / 36 | App 入口与 DB consumer 分别绑定 `RQ-LEG-*` |
| SDK auth/public 边界 | 61 | 精确文件 pathspec；private/full 命中只作负边界 |
| legacy instrument header capture / status-error | 0 / 9 | header 缺口与 error 现状分别绑定 `EXSDK-LEG-002/006` |
| I1 相关 legacy `f64` | 8 | 均在延期 live example；新 I1 文件要求 0 |

结论：冻结 legacy 语义与调用点已形成可复现闭包；当前工作树源码存在不等于验证通过。

## 5. 实施放置声明

- Owner：Exchange SDK；
- capability：provider public instrument protocol；
- slice：read-only Query Adapter/facade；
- provider endpoint：Binance USDⓈ-M exchangeInfo、OKX SWAP public instruments；
- Domain/Use Case/Port：不创建；这是 SDK 协议能力，不伪造空 trait 或万能 service；
- Adapter：`binance_rs`、`okx_rs` HTTP client + provider DTO；
- 公开面：root 两个具体 public-only client 与 typed response/failure；
- persistence/transaction/outbox：无；
- recovery owner：Market；F4C 是已登记的后继实施切片，不是 Owner 名；
- Release Unit / binary / deploy：无。

## 6. 工作树与范围证据

开始 I1 前，`crypto_exc_all` 已存在用户的 `README.md` 修改。当前工作树还已落入 I1
源码与测试文件；本文档按实际文件名收敛 Manifest，但不把“文件存在”冒充测试通过。
`README.md`：

- 不在 I1 allowed paths；
- 不会被覆盖、暂存、回退或加入 Manifest allowlist；
- 会使主工作树 P2 diff-scope 检查按设计 fail closed。

完整主工作树 migration-check 必须如实包含全部 diff。精确 I1 patch 的独立验证如需
linked worktree，只能用于证明 I1 patch 自身，不能隐藏主工作树 `README.md` 越界。

## 7. 验证结果

| 验证 | 当前结果 | 备注 |
| --- | --- | --- |
| Manifest schema / authority / pinned artifact / profile | PASS | 2026-07-30 `migration-check` 对应阶段均为 `errors=0 warnings=0` |
| Global Registry | PASS | 2026-07-30 `migration-registry-check`：`errors=0 warnings=0` |
| Binance contract | PASS | `instrument_api_tests` 4/4：exact path/no query/no auth、DTO/unknown/decimal、provider error 与条件限频证据 |
| OKX contract | PASS | `public_instrument_tests` 7/7：固定 SWAP query/no auth、envelope/坏行/空集、429/5xx 与安全 header |
| Logical public facade | PASS | `public_instrument_facade` 2/2；只证明两个具体 client 的逻辑方法面，不声称物理 crate 隔离 |
| Decimal wire / auth source guard | PASS | 新 facade/API/DTO 中业务 `f64` 类型或转换 0 命中，认证与模拟交易 header 0 命中 |
| Existing-feature build | PASS | root `binance`、`okx-public-market`、两者组合及 OKX `public-market` 定向 `cargo check` 均成功 |
| Focused Clippy | PASS | OKX public 子集严格 `-D warnings`；Binance/root 对已定位的 legacy lint 使用显式 `-A` 后，本轮代码无新增 lint |
| Strict workspace test | BLOCKED（legacy） | 原命令仅 6 个既有 OKX live test 因缺 `OKX_API_KEY` 失败；明确 skip 这 6 项后，全 workspace/all-targets/all-features PASS |
| Strict workspace Clippy | BLOCKED（legacy） | 原命令在既有 OKX/Bybit/Binance 文件失败；OKX full 当前报告 143 个 legacy lint，本轮未扩大范围清理 |
| Format / diff check | PASS | `cargo fmt --all -- --check` 与 I1 范围 `git diff --check` 通过 |
| File budget | PASS（有 baseline warning） | 新生产文件最大 176 行，修改后的 provider client 为 362/595 行；脚本只报告 4 个既有 1000 行目标 warning，均低于 2000 硬上限 |
| Required-artifact / target closure | PASS | 实际测试文件名已精确登记；当前 checker 未报告缺失工件或 target closure 错误 |
| Clean I1 patch P2 migration-check | PASS | 从 `c17ba151...` 建立临时干净 clone，仅应用 30 个 I1 path（含迭代记录）；所有阶段 `errors=0 warnings=0` |
| 主工作树完整 P2 migration-check | FAIL（预期） | 唯一错误为既有 `README.md` 命中 forbidden path；`errors=1 warnings=0` |

初始文档预检时，源码工件尚未全部落入工作树，因此“required artifacts + README”
属于当时的预期失败集合。当前源码已并行落地并按实际文件名登记；2026-07-30 重跑的
完整 checker 不再报告 required-artifact 错误，只剩 `README.md` 范围阻塞。两次状态
不能混写成同一次测试结果。

本轮机器验证冻结在：

- base revision：`c17ba15185a337e03df5dfe4ecf08e7fd3e8a380`；
- source/test patch SHA-256：
  `04d3d727591f36e45990eb59be68e0cbbdd09f85b48e0711cee4dfdbf8f1c505`；
- patch hash 明确排除会自引用的 Manifest/Evidence 文档；
- 25 个 source/test output artifact 的逐文件 SHA-256 已写入 Manifest。

严格 workspace test 中被排除后复跑的 6 个 legacy 名称为
`test_get_balances`、`test_transfer`、`test_get_ticker`、`test_get_candles`、
`test_get_economic_calendar`、`test_get_instruments`。这些测试读取账户凭证或访问 live
endpoint；I1 contract tests 全部使用 disposable local mock，未访问真实交易所、未执行
任何账户读取或 mutation。

## 8. 未完成项与 Verdict

- typed DTO、public client、结构化 HTTP/provider evidence、root facade 与 mock contract
  tests 已实现并通过 focused verification；
- strict workspace Clippy 的 legacy lint，以及 6 个未正确隔离的 legacy live test，仍是
  仓库级基线阻塞；本轮没有越界修改这些无关旧模块；
- Market F4C、consumer migration、runtime、数据库、scheduler、readiness、cutover 均不在 I1；
- P2 没有 current Verdict，I1 即使本地测试通过也最高保持 `implementing`；
- 技术结论：`I1 contract 与 clean patch 已验证；全仓基线门禁未闭合`；
- Cutover eligibility：`不适用`；
- Legacy delete eligibility：`不允许`；
- 是否含敏感数据：`否`。
