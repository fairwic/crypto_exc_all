# I1 冻结调用点搜索闭包

## 1. 复现边界

- SDK 基线：`crypto_exc_all@c17ba15185a337e03df5dfe4ecf08e7fd3e8a380`
- Core 基线：`rust_quant@30789257dfea817cbde91d38fe91fd60f638c478`
- 命中数按 `git grep` 输出行计数，不按文件数计数。
- 下列命令分别在 `crypto_exc_all` 或 `rust_quant` 仓库根目录执行；所有
  pathspec 都是冻结且可复制的，不读取当前工作树。

## 2. Instrument endpoint

```bash
git grep -n -E 'get_exchange_info|get_instruments|exchangeInfo|public/instruments|market/instruments' c17ba15185a337e03df5dfe4ecf08e7fd3e8a380 -- '*.rs'
```

结果为 11 行：

| 冻结文件 | 行 | 矩阵 ID |
| --- | --- | --- |
| `binance_rs/examples/live_post_only_order.rs` | `24,115` | `EXSDK-LEG-008` |
| `binance_rs/src/api/market/market_api.rs` | `39,40` | `EXSDK-LEG-001` |
| `binance_rs/tests/expanded_api_tests.rs` | `27,79` | `EXSDK-LEG-001/008` |
| `bybit_rs/src/api/market/market_api.rs` | `54` | `EXSDK-LEG-007`（延期 provider 的保留证据） |
| `okx_rs/src/api/market/market_api.rs` | `172` | `EXSDK-LEG-004` |
| `okx_rs/src/api/public_data/public_data_api.rs` | `75,266,268` | `EXSDK-LEG-003/005` |

## 3. Core 直接采集与消费者

### 3.1 Binance

```bash
git grep -n -E 'fapi/v1/exchangeInfo|\.get_exchange_info|load_current_live_crypto_perpetuals|exchange_info_filters' 30789257dfea817cbde91d38fe91fd60f638c478 -- '*.rs'
```

结果为 19 行：

| 冻结文件 | 行 | 矩阵 ID |
| --- | --- | --- |
| `crates/rust-quant-cli/src/app/binance_eth_micro_live_validation.rs` | `212` | `RQ-LEG-010` |
| `crates/rust-quant-cli/src/app/binance_eth_micro_live_validation/binance_futures_http.rs` | `53,54` | `RQ-LEG-010` |
| `crates/rust-quant-cli/src/app/market_cross_exchange_basis_panel/binance_funding.rs` | `2,84` | `RQ-LEG-008` |
| `crates/rust-quant-cli/src/app/market_cross_exchange_basis_panel/binance_klines.rs` | `128,205,264,268` | `RQ-LEG-008` |
| `crates/rust-quant-cli/src/app/market_cross_exchange_basis_panel/binance_positioning.rs` | `2,87` | `RQ-LEG-008` |
| `crates/rust-quant-cli/src/app/market_flow_flip_reversal_research/metrics.rs` | `292,390,394` | `RQ-LEG-009` |
| `crates/rust-quant-cli/src/app/market_orderbook_depth_panel/book_depth.rs` | `204,492,497` | `RQ-LEG-009` |
| `crates/services/src/market/exchange_symbol_sync_service.rs` | `106` | `RQ-LEG-001/002` |
| `crates/services/tests/binance_live_eth_micro_order_smoke_contract.rs` | `127` | `RQ-LEG-010` |

### 3.2 OKX

```bash
git grep -n -E '/api/v5/public/instruments|STRICT_STATIC|load_current_live_contract_values|fetch_live_okx_swap_symbols' 30789257dfea817cbde91d38fe91fd60f638c478 -- '*.rs'
```

结果为 10 行：

| 冻结文件 | 行 | 矩阵 ID |
| --- | --- | --- |
| `crates/rust-quant-cli/src/app/all_market_candle_volume_monitor/warmup.rs` | `56,82,87` | `RQ-LEG-005` |
| `crates/rust-quant-cli/src/app/okx_historical_15m_backfill.rs` | `3,340` | `RQ-LEG-006` |
| `crates/rust-quant-cli/src/app/okx_historical_universe.rs` | `249,312,353,357` | `RQ-LEG-006/007` |
| `crates/services/src/market/exchange_symbol_sync_service.rs` | `112` | `RQ-LEG-001/002` |

### 3.3 同步运行入口

```bash
git grep -n -E 'run_exchange_symbol_sync_from_env|run_exchange_symbol_sync_worker_from_env|handle_exchange_symbol_sync_body|ExchangeSymbolSyncService::from_env' 30789257dfea817cbde91d38fe91fd60f638c478 -- crates/rust-quant-cli/src/app/bootstrap.rs crates/rust-quant-cli/src/app/exchange_symbol_sync.rs crates/rust-quant-cli/src/app/internal_server.rs crates/rust-quant-cli/src/app/market_worker.rs crates/rust-quant-cli/src/bin/sync_exchange_symbols.rs
```

结果为 15 行，全部绑定 `RQ-LEG-004`；其中
`exchange_symbol_sync.rs:99,162` 也绑定 `RQ-LEG-001`：

| 冻结文件 | 行 |
| --- | --- |
| `crates/rust-quant-cli/src/app/bootstrap.rs` | `5,161,165,363,371,392` |
| `crates/rust-quant-cli/src/app/exchange_symbol_sync.rs` | `99,162` |
| `crates/rust-quant-cli/src/app/internal_server.rs` | `24,916,941,1104` |
| `crates/rust-quant-cli/src/app/market_worker.rs` | `22` |
| `crates/rust-quant-cli/src/bin/sync_exchange_symbols.rs` | `4,15` |

### 3.4 `exchange_symbols` 生产消费者

```bash
git grep -n -E 'exchange_symbols' 30789257dfea817cbde91d38fe91fd60f638c478 -- 'crates/infrastructure/src/**/*.rs' 'crates/rust-quant-cli/src/app/*.rs' 'crates/rust-quant-cli/src/app/**/*.rs' 'crates/rust-quant-cli/src/bin/*.rs' 'crates/rust-quant-cli/src/bin/**/*.rs' 'crates/services/src/*.rs' 'crates/services/src/**/*.rs'
```

结果为 36 行：

| 冻结文件 | 行 | 矩阵 ID |
| --- | --- | --- |
| `crates/infrastructure/src/repositories/exchange_symbol_repository.rs` | `195,254,290,302,327,337,374,386` | `RQ-LEG-003/011` |
| `crates/rust-quant-cli/src/app/all_market_candle_volume_monitor/warmup.rs` | `40,70` | `RQ-LEG-005/011` |
| `crates/rust-quant-cli/src/app/market_velocity_backfill.rs` | `514,859,905,946,1448,1449,1507` | `RQ-LEG-003/011` |
| `crates/rust-quant-cli/src/app/market_velocity_event_backtest/data.rs` | `344,377,989` | `RQ-LEG-011` |
| `crates/rust-quant-cli/src/app/market_velocity_kline_scanner.rs` | `281,310,311,312,313,314,497` | `RQ-LEG-011` |
| `crates/rust-quant-cli/src/app/market_velocity_live_handoff/candidates.rs` | `43,320,321` | `RQ-LEG-011` |
| `crates/rust-quant-cli/src/bin/vegas_cross_asset_portfolio_replay/live_universe.rs` | `19` | `RQ-LEG-011` |
| `crates/rust-quant-cli/src/bin/vegas_cross_asset_portfolio_replay/universe_coverage.rs` | `40,47,57,88` | `RQ-LEG-011` |
| `crates/services/src/rust_quan_web/execution_order_filters.rs` | `64` | `RQ-LEG-011` |

## 4. SDK 边界补充搜索

### 4.1 冻结 feature 事实

```bash
git grep -n -E '^full-sdk|^okx-public-market|^okx =|^binance =' c17ba15185a337e03df5dfe4ecf08e7fd3e8a380 -- Cargo.toml
```

结果为 `Cargo.toml:21,22,23,24` 共 4 行，全部绑定
`EXSDK-LEG-007`。它证明 root `binance` 仍含 `full-sdk`，因此 I1 只能声明新增
facade 的逻辑 public 方法面，不能声明物理依赖闭包隔离。

### 4.2 认证/公共 transport

```bash
git grep -n -E 'send_request|send_public_request|new_public|from_env|OK-ACCESS-|X-MBX-APIKEY' c17ba15185a337e03df5dfe4ecf08e7fd3e8a380 -- src/lib.rs src/public_market.rs binance_rs/src/client.rs binance_rs/src/api/market/market_api.rs binance_rs/tests/expanded_api_tests.rs binance_rs/examples/live_post_only_order.rs okx_rs/src/client.rs okx_rs/src/api/market/market_api.rs okx_rs/src/api/public_data/public_data_api.rs
```

结果为 61 行：

| 冻结文件 | 行 | 矩阵 ID |
| --- | --- | --- |
| `binance_rs/examples/live_post_only_order.rs` | `21,22` | `EXSDK-LEG-008` |
| `binance_rs/src/api/market/market_api.rs` | `28,29,35,42,56,63,73,83,93,101,150` | `EXSDK-LEG-001` |
| `binance_rs/src/client.rs` | `27,30,31,34,35,65,75,101,115,118,133` | `EXSDK-LEG-002` |
| `binance_rs/tests/expanded_api_tests.rs` | `18` | `EXSDK-LEG-001/008` |
| `okx_rs/src/api/market/market_api.rs` | `26,27,42,53,79,113,147,162,189,210,218` | `EXSDK-LEG-004` |
| `okx_rs/src/api/public_data/public_data_api.rs` | `25,26,70,97,138,151,166,196,223,250,267,274` | `EXSDK-LEG-003/006` |
| `okx_rs/src/client.rs` | `56,58,101,102,108,109,138,160,161,162,163,180` | `EXSDK-LEG-003/006` |
| `src/public_market.rs` | `107` | `EXSDK-LEG-007` |

### 4.3 Header、状态与错误

```bash
git grep -n -i -E 'retry-after|x-mbx-used-weight|response\.headers|headers\(\)' c17ba15185a337e03df5dfe4ecf08e7fd3e8a380 -- binance_rs/src/client.rs binance_rs/src/error.rs binance_rs/src/api/market/market_api.rs binance_rs/tests/expanded_api_tests.rs okx_rs/src/client.rs okx_rs/src/error.rs okx_rs/src/api/market/market_api.rs okx_rs/src/api/public_data/public_data_api.rs
```

结果为 0 行，绑定 `EXSDK-LEG-002/006`：冻结 instrument 路径没有响应
header capture。I1 只能保留响应实际携带的 header，不能保证 provider 一定返回某个
quota header。

```bash
git grep -n -E 'RateLimit|response\.status|StatusCode' c17ba15185a337e03df5dfe4ecf08e7fd3e8a380 -- binance_rs/src/client.rs binance_rs/src/error.rs okx_rs/src/client.rs okx_rs/src/error.rs
```

结果为 9 行：`binance_rs/src/client.rs:137` 绑定 `EXSDK-LEG-002`；
`okx_rs/src/client.rs:9,209,213,248` 与
`okx_rs/src/error.rs:73,100,111,122` 绑定 `EXSDK-LEG-006`。

### 4.4 I1 相关 legacy `f64`

```bash
git grep -n -E 'parse::<f64>|as_f64|f64' c17ba15185a337e03df5dfe4ecf08e7fd3e8a380 -- src/public_market.rs binance_rs/src/api/market/market_api.rs binance_rs/src/dto/market/market_dto.rs binance_rs/examples/live_post_only_order.rs binance_rs/tests/expanded_api_tests.rs okx_rs/src/api/market/market_api.rs okx_rs/src/api/public_data/public_data_api.rs okx_rs/src/dto/market/market_dto.rs
```

结果为 8 行，均在
`binance_rs/examples/live_post_only_order.rs:18,105,181,186,190,194,198,207`，
绑定 `EXSDK-LEG-008`。该 live example 延期且不运行；I1 新 DTO/facade 文件的要求是
0 命中。

## 5. 闭包结论

- 上述 11、19、10、15、36、4、61、0、9、8 行结果均已逐文件、逐行绑定
  `EXSDK-LEG-*` 或 `RQ-LEG-*`；
- 测试/example 只证明 contract 或遗留调用存在，不冒充生产 caller；
- Bybit 命中只证明延期 provider 必须保留，不扩大 I1 的 Binance/OKX V1 范围；
- 当前工作树实现与测试结果不属于冻结搜索；它们必须由独立命令验证，不能回填为
  `tested_revision`、output hash 或最终 Verdict。
