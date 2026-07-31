# K2 Binance/OKX 公共 transport 调用点闭包

## 1. 冻结基线

| 仓库 | Commit |
| --- | --- |
| `crypto_exc_all` | `a7369ff9a032038c9f74c10a23b482bfa127b0f9` |
| `rust_quant_alpha` | `d9cc91a983f80abd8ae95d11cf1b6d6ff07e335f` |
| `rust_quant` governance / registration | `8cbae84e109cf36869947e74b1ec77889ec63fb1` |

搜索只读取 committed object；用户已有 `README.md` 修改不作为基线，也不进入 K2。

## 2. SDK public constructor 闭包

```bash
git grep -n -E \
  'BinanceClient::new_public|OkxClient::new_public|OkxPublicInstruments::(new|with_base_url)|BinanceUsdmPublic(Kline|Instrument)Client::new|Okx(PublicMarket|SwapPublicInstrument)Client::new' \
  a7369ff9a032038c9f74c10a23b482bfa127b0f9 -- '*.rs'
```

结果为 32 行，逐类闭包如下：

| 类别 | committed 调用点 | 处置 / 矩阵 |
| --- | --- | --- |
| Binance provider library | `binance_rs/src/api/market/market_api.rs:29`、`api/websocket/websocket_api.rs:69` | 保留环境型 constructor；`K2-001` |
| Binance example | `binance_rs/examples/live_post_only_order.rs:21` | 保留，K2 不改变示例运行配置；`K2-001` |
| Binance provider tests | `api_tests`、`announcements_api_tests`、`expanded_api_tests`、`instrument_api_tests`、`kline_api_tests`、`okx_gap_api_tests` | 保留兼容回归；`K2-001` |
| Binance root facade | `src/public_market/binance.rs:25`、`src/public_instrument/binance.rs:27` | 改为显式 default/with_transport；`K2-002/003` |
| OKX provider library | `okx_rs/src/api/public_data/instrument_api.rs:21,29` | 保留 `new/with_base_url`，内部改用显式 config；`K2-004/005` |
| OKX root facade | `src/public_market.rs:142`、`src/public_instrument/okx.rs:28-30` | 改为显式 default/with_transport；`K2-004/005` |
| root/provider contract tests | `tests/*public*` 与 `okx_rs/tests/public_instrument_tests.rs` | 保持原 config shape 与请求 contract；`K2-006/010` |

文档注释命中不作为调用方。测试命中证明兼容面，但不能替代 Alpha 生产组合根。

## 3. Ambient 配置闭包

```bash
git grep -n -E \
  'Config::from_env|BINANCE_(API_URL|API_TIMEOUT_MS|PROXY_URL)|DEFAULT_PUBLIC_API_(URL|TIMEOUT_MS)' \
  a7369ff9a032038c9f74c10a23b482bfa127b0f9 -- '*.rs'
```

结果为 33 行：

- Binance `Config::from_env` 在 market/WebSocket/asset/announcement/example 等既有入口有真实
  使用，不能以 K2 一次性删除；
- K2 只关闭 root public Kline/instrument facade 对
  `BINANCE_API_URL/BINANCE_API_TIMEOUT_MS/BINANCE_PROXY_URL/dotenv` 的隐式读取；
- root full SDK 的 `SdkConfig::from_env`、signed adapter 与其他 provider 的环境构造保持
  forbidden，不顺手重构；
- OKX 的两个公共默认常量从大 `client.rs` 移入独立 `public_transport.rs`，full credential
  `CONFIG` 不进入新类型。

## 4. Alpha consumer 闭包

```bash
git grep -n -E \
  'BinanceUsdmPublic(Kline|Instrument)(Client|Config)|Okx(PublicMarket|SwapPublicInstrument)(Client|Config)' \
  d9cc91a983f80abd8ae95d11cf1b6d6ff07e335f -- '*.rs'
```

结果为 17 行：

- `apps/market-worker/src/wiring.rs:29,46` 是当前唯一实际构造 root client 的 Alpha
  组合根，只构造 OKX/Binance Kline client；
- 四个 Gateway source 类型只接收已经构造好的 client，不拥有 endpoint、timeout、proxy
  或环境配置；
- `source_profile_id` 当前在 App 构造后交给 Gateway，但没有与 transport/egress
  形成同一配置证据。

K2 不修改 Alpha。后继 Market source-profile Manifest 必须把该两个构造点切换到
`with_transport`，同时冻结 `source_profile_id + endpoint + egress_identity + timeout +
PublicQuotaKey`；在该后继提交前不得宣称生产 transport ready。

## 5. 闭包结论

- 32 行 public constructor、33 行 ambient 配置和 17 行 Alpha facade 命中均已分类；
- direct legacy constructor 有真实调用方，删除条件已进入 Manifest；
- K2 的实施范围只覆盖两家 provider transport 类型、root facade 显式入口和 contract
  tests；
- Market profile/egress/quota、固定 Key、其他 provider 和 runtime cutover 都有明确延期
  Gate，不会因切片缩小而静默丢失。
