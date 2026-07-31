# K2 Binance/OKX 显式公共 Transport Evidence

## 1. 身份与状态

| 字段 | 值 |
| --- | --- |
| Program / Owner child | `MP-market-data-foundation-v1 / MIG-EXSDK-K2-BINANCE-OKX-EXPLICIT-PUBLIC-TRANSPORT-V1 / Exchange SDK` |
| Owner repository | `crypto_exc_all` |
| Registration revision | `rust_quant@8cbae84e109cf36869947e74b1ec77889ec63fb1` |
| SDK base revision | `crypto_exc_all@a7369ff9a032038c9f74c10a23b482bfa127b0f9` |
| Repository profile | `rust_quant_alpha@d9cc91a983f80abd8ae95d11cf1b6d6ff07e335f` |
| 技术状态 | `implementing` |
| 模式 | `behavior_change` |
| Cutover / production mutation | 不需要 / 禁止 |
| Machine Verdict | 不创建；前置 K1 尚无 current-revision Verdict |

## 2. 当前判断

- Binance root public facade 的 endpoint override 不能消除 timeout/proxy/dotenv 的 ambient
  输入，同一 `source_profile_id` 可能落到不同 transport；
- OKX 当前没有等价的显式 timeout/proxy 入口；
- 现有底层环境型 constructor 有真实 committed caller，K2 只能新增显式能力并改造目标
  root facade，不能删除 legacy；
- 固定平台 API Key 不属于 transport config。当前 Kline/instrument endpoint 均匿名，
  K2 不读取任何 credential；
- Market source profile、egress identity 与共享 quota 的绑定仍是后继 Gate，SDK 完成不
  等于 runtime ready。

## 3. 实施放置声明

- Owner：Exchange SDK；
- capability：provider `public_transport`；
- slice：公共只读 SDK transport behavior change；
- entry：provider `new_public_with_transport` 与 root facade `with_transport`；
- Model：两个 provider-specific immutable config value；
- Domain / Use Case / Port：不创建；
- Adapter：reqwest public-only client construction；
- public surface：endpoint、毫秒 timeout、可选 proxy；
- transaction / persistence / Outbox：无；
- recovery owner：Market；
- App / binary / Release Unit：无；
- file budget：新生产文件目标 `<150` 行，测试文件 `<250` 行；OKX 大 client 通过抽取而非
  继续堆积。

## 4. 工作树边界

开始 K2 前 `crypto_exc_all` 仅有用户既存 `README.md` 修改。它命中 Manifest forbidden
path，必须保留、不覆盖、不暂存；K2 最终使用精确 allowlist patch/临时干净 checkout
验证自身范围。

## 5. Preflight

| 检查 | 结果 |
| --- | --- |
| Registry 两阶段 registration | 已提交，child 为 `not_created` |
| architecture baseline 与规范 hash | 已固定到 `rust_quant@8cbae84e...` |
| predecessor K1 Manifest/Evidence | 已固定到 `crypto_exc_all@a7369ff...`，无 Verdict |
| committed callsite closure | 已完成：constructor 32 行、ambient 配置 33 行、Alpha facade 17 行 |
| legacy 语义处置 | 已写入 `business-semantics-matrix.md` |
| production/live network | 未授权且本切片不需要 |

## 6. TDD 与验证

### 6.1 RED / GREEN

- RED：先添加 provider/root contract tests，`bnb_rs` 编译明确缺少
  `BinancePublicTransportConfig` 与 `BinanceClient::new_public_with_transport`；
- 二次安全 RED：非法 endpoint userinfo 在 config `Debug` 中仍会暴露；实现改为仅对
  已通过 endpoint 安全校验的值显示原文，其余统一显示脱敏占位符；
- GREEN：新增两个 provider config、provider constructor 与四个 root
  `with_transport` 后，K2 focused matrix 13/13 通过；
- Provider 测试用本地延迟 HTTP server 证明 timeout 作用于真实 request，并验证
  默认 endpoint/5000ms/no-proxy、非法 scheme/userinfo/query/fragment、zero timeout、
  非法 proxy 与错误脱敏；
- Root 测试在隔离子进程注入非法 `BINANCE_API_URL`、`BINANCE_API_TIMEOUT_MS` 和
  `BINANCE_PROXY_URL`，四个 Kline/instrument facade 仍只使用调用方显式 transport；
- 既有 Binance/OKX Kline 与 instrument contract tests 继续证明 endpoint、query、
  匿名 header、wire DTO 和 provider evidence 没有改变。

### 6.2 当前验证矩阵

| 验证 | 当前结果 | 备注 |
| --- | --- | --- |
| Provider public transport | PASS | Binance 2/2；OKX minimal `public-market` 2/2 |
| Root explicit transport / ambient isolation | PASS | 1/1，父子进程均通过 |
| 既有 Kline regression | PASS | Binance 2/2；OKX 2/2 |
| 既有 instrument regression | PASS | 4/4 |
| Focused Clippy | PASS | K2 provider/root lib 与目标 test 在 `-D warnings` 下通过；只对未修改的 Binance `AlgoOrderIdRequest` 历史 lint 做定点豁免 |
| API docs / format / diff / source guards | PASS | 三个 package 文档成功；K2 source 无 ambient env、credential、runtime owner 逻辑 |
| File budget | PASS（baseline warning） | 新生产文件 11/101/111 行；OKX client 595→579；脚本仅报告四个既有 1000 行以上文件 warning |
| Workspace compile | PASS | `cargo check --locked --workspace --all-targets --all-features` |
| Workspace tests | BLOCKED（legacy/environment） | 清理可再生 `target` 后完成全量编译并运行；最终只有 6 个未修改的 OKX live-credential tests 因缺 `OKX_API_KEY` 失败，其中包含 asset transfer，未注入凭证或联网 |
| Workspace Clippy | BLOCKED（legacy） | 未修改的 OKX full-sdk、Bybit 与 Binance 文件存在历史 lint；K2 精确 Clippy 已通过 |
| Registry global shape / identity / graph | PASS | `errors=0 warnings=0` |
| Clean exact-patch migration-check | PASS | disposable clean clone 只应用 22 个 K2 allowlist 路径：`errors=0 warnings=0` |
| Main worktree migration-check | BLOCKED（pre-existing） | 唯一 scope error 是用户既有 `README.md` 命中 forbidden path |
| Production/live mutation | 未执行 | 无真实交易所网络、凭证、数据库、下单或部署 |

上述 workspace 阻塞文件的 working-tree blob 与
`crypto_exc_all@a7369ff9a032038c9f74c10a23b482bfa127b0f9` 完全一致，K2 不通过顺手修改
旧 full-sdk/live tests 来制造表面全绿。

### 6.3 冻结产物

测试冻结在 SDK base
`a7369ff9a032038c9f74c10a23b482bfa127b0f9` 加 source/test patch SHA-256
`529c4561b1548defd644fd3ba108a33f24c433a4f4c643e9dffd98d137d38b3a`。
该 patch 为 54,380 bytes，使用独立临时 Git index 只收录 17 个 K2 code/test/lock
路径，排除 `README.md`、自引用迁移文档与 `task_plan.md`。18 个
code/test/lock/task-plan output artifact 的逐文件 SHA-256 已写入 Manifest。

## 7. 未完成与不宣称

- 前置 K1 没有 current-revision Verdict，K2 不得进入 `verified/completed`；
- K2 尚未提交；提交后才可用 immutable Manifest/Evidence hash 把 Registry 从
  `not_created` 更新为 `created`；
- Alpha `market-worker` 尚未把 `source_profile_id + endpoint + egress_identity +
  timeout + PublicQuotaKey` 绑定到 `with_transport`，不能宣称 runtime ready；
- 未运行真实 OKX/Binance 网络；
- 未连接数据库、未写 schema/分表；
- 未修改 Alpha Market worker、source profile、quota、scheduler、compose 或部署；
- 未运行受跟踪 CI/CD；
- 未创建 Verdict，技术状态不得超过 `implementing`。
