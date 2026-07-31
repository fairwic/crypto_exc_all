# K2 Binance/OKX 显式公共 transport 业务语义矩阵

## 1. 结论

K2 只把公共只读 HTTP transport 的 endpoint、timeout 与 proxy 从隐式/硬编码状态变成
调用方显式输入。它不拥有 Market 的 source profile、egress identity、公共配额、调度、
重试、数据 finality 或固定服务凭证，也不修改任何 K 线/instrument provider 协议。

现有底层环境型构造器已经有真实 library、WebSocket、example 和测试调用方，因此本轮
不删除；目标 root public facade 则必须停止继承 ambient env，供后继 Market 以可审计
source profile 显式装配。

## 2. 四类认知

### 已知的已知

- `BinanceClient::new_public()` 会调用 `Config::from_env()`，并触发 dotenv 向上搜索；
- Binance root Kline/instrument facade 只覆盖 `api_url`，timeout/proxy 仍由 ambient env
  决定；
- OKX `new_public()` 固定官方 endpoint、5000ms timeout 且没有 proxy 配置；
- Alpha `market-worker` 当前只构造两家 Kline root facade，并把 `source_profile_id`
  传给 Gateway；transport 尚未与该 identity 绑定；
- Kline 与 instrument 都是匿名公共 endpoint，不需要平台固定 Key，更不能使用用户
  credential。

### 已知的未知

- 生产 `source_profile_id -> endpoint/egress/timeout/proxy ref` 的配置存储与注入方式尚未
  冻结；
- 多实例共享 `PublicQuotaKey` 的协调、scheduler、lease 与 `next_eligible_at` 尚未实施；
- 未来某个公共 endpoint 是否会要求平台 `MarketDataAccessCredential`，必须按当时官方
  权限和只读 evidence 另行判断。

### 未知的已知

- 现有 root config 被 struct literal 使用；直接增加字段会让已提交 consumer/test
  无法编译，因此显式 transport 必须使用独立构造入口；
- `okx_rs/src/client.rs` 已接近文件预算 Error 线，继续内联配置会制造大文件，transport
  必须进入独立 provider module；
- proxy URL 可能包含认证信息，自动派生 `Debug` 或透传 parse error 会把 secret 写进
  日志，因此新 config/error 必须脱敏；
- 两家配置字段相似不代表应合并成通用 enum；独立 provider 类型才能保留未来差异。

### 未知的未知

- 交易所可能变更地区域名、代理策略、匿名访问政策或 TLS/限频行为；
- 代理/NAT 共享出口可能让配置正确但实际 quota identity 错配；
- 某些代理只在连接阶段失败，构造成功不能被当作 source readiness；
- endpoint 重定向或代理认证错误可能产生含基础设施细节的底层错误，调用方仍需执行
  日志脱敏。

## 3. 逐项处置

| ID | 冻结事实 | 处置 | K2 目标 / 后继 Owner |
| --- | --- | --- | --- |
| `K2-001` | Binance `new_public()` 读取 endpoint/timeout/proxy 与 dotenv | 兼容保留 | direct legacy caller 保持；目标 root facade 改用显式 transport |
| `K2-002` | Binance root Kline 只覆盖 endpoint，隐式继承 timeout/proxy | 优化 | `new(config)` 映射 provider 默认值；`with_transport` 接受完整显式配置 |
| `K2-003` | Binance root instrument 与 Kline 重复同一隐式构造 | 优化 | 共用 `BinancePublicTransportConfig`，endpoint API 仍各自独立 |
| `K2-004` | OKX public client 固定 endpoint/5000ms | 扩充 | `OkxPublicTransportConfig` 显式声明同三项 transport 输入 |
| `K2-005` | OKX instrument 只有 `new/with_base_url` | 兼容扩充 | 保留旧入口，新增 `with_transport` |
| `K2-006` | 两家 root config 只有 `api_url: Option<String>` 且被 struct literal 使用 | 保留 | 不加字段、不私有化；新增独立 transport type 避免 source break |
| `K2-007` | endpoint 可延迟到 request 时才报错 | 优化 | 非 http/https、userinfo、query、fragment 在构造期 fail-closed |
| `K2-008` | timeout=0 可形成不可解释的立即超时 | 优化 | 构造期返回 provider `ConfigError` |
| `K2-009` | proxy parse error/Debug 可能暴露含认证信息的 URL | 优化 | config Debug 只显示 `proxy_configured`，错误不回显原值 |
| `K2-010` | public endpoint 不发送认证 header | 保留 | transport config 无 credential 字段；现有 header contract test继续通过 |
| `K2-011` | SDK 单次请求、不 retry/sleep/quota accounting | 保留 | K2 只负责 transport construction；恢复归 Market |
| `K2-012` | Alpha source profile 与 SDK transport 尚未绑定 | 延期 | 后继 Market source-profile Gate 显式装配 `with_transport` |
| `K2-013` | 固定平台 Key 可能用于未来部分公共 endpoint | 延期 | 只有真实 endpoint 需要时另建 `MarketDataAccessCredential` capability |
| `K2-014` | OKX 当前 reqwest public feature 未开启 SOCKS | 优化 | 只增加 transport feature，与账户/signature feature 隔离 |
| `K2-015` | 其他 provider 也存在环境型 public constructor | 延期 | Bitget/Bybit/Gate/Hyperliquid 各自另立 provider Manifest，不纳入 V1 |

## 4. Transport V1 冻结

| Provider | Config 类型 | 默认 endpoint | timeout 单位 | proxy | credential |
| --- | --- | --- | --- | --- | --- |
| Binance USD-M | `BinancePublicTransportConfig` | `https://fapi.binance.com` | 毫秒，默认 `5000` | 可选 HTTP(S)/SOCKS | 禁止 |
| OKX | `OkxPublicTransportConfig` | `https://www.okx.com` | 毫秒，默认 `5000` | 可选 HTTP(S)/SOCKS | 禁止 |

配置只描述 transport。`source_profile_id`、`egress_identity`、secret reference、
`PublicQuotaKey`、retry、scheduler 和 readiness 不进入这两个类型。

## 5. 明确不属于 K2

- Market worker plan、source profile 存储与 App 环境变量解析；
- proxy secret manager、egress/NAT inventory 与 quota coordinator；
- Kline/instrument endpoint、query、typed DTO、Decimal 或 finality；
- 固定平台 API Key、用户 API Key、账户读取、私有流和 mutation；
- scheduler、lease、retry、backoff、cursor、run ledger 与数据库；
- rust_quant_alpha runtime wiring、compose、部署、cutover 与 CI/CD；
- 其他四家以上 provider 的 transport 统一。
