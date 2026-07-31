# MIG-EXSDK-K1 Binance USD-M 公共 K 线调用点闭包

## 1. 冻结基线

| 仓库 | Commit |
| --- | --- |
| `crypto_exc_all` | `db467416bb4c3d5f895e7f16a61b32768e79a61b` |
| `rust_quant` | `f4acef65caca988b8ee9cd5ef9f1f4dd9d3e1c82` |

所有命令读取 committed object，不把当前工作树结果混入 legacy 事实。

## 2. SDK endpoint、mapper 与测试

```bash
git grep -n -E 'get_klines|KlineRequest|binance_candles_from_value|/fapi/v1/klines' \
  db467416bb4c3d5f895e7f16a61b32768e79a61b -- '*.rs'
```

结果为 14 行：

| 冻结文件 | 行 | 矩阵 ID |
| --- | --- | --- |
| `binance_rs/src/api/market/market_api.rs` | `64,160,168` | `EXSDK-K-001/002` |
| `binance_rs/tests/expanded_api_tests.rs` | `2,46,86` | `EXSDK-K-001/002` |
| `src/adapters/binance.rs` | `28,164,177,181,1469` | `EXSDK-K-003/004/005/006` |
| `tests/external_consumer_tests.rs` | `181,362,376` | `EXSDK-K-009` |

测试命中只证明已有 contract，不冒充生产 caller。K1 不修改
`src/adapters/binance.rs`，因为 root unified mapper 与消费者切换属于 Market F2B。

## 3. Core REST 生产链

```bash
git grep -n -E \
  'fetch_candles_from_crypto_exc_all|CandleQuery|\.candles\(|sync_kline_request|checked_add\(1\)' \
  f4acef65caca988b8ee9cd5ef9f1f4dd9d3e1c82 -- \
  crates/services/src/market/mod.rs \
  crates/rust-quant-cli/src/app/internal_server/kline_sync_section.rs
```

结果为 7 行：

| 冻结文件 | 行 | 矩阵 ID |
| --- | --- | --- |
| `crates/rust-quant-cli/src/app/internal_server/kline_sync_section.rs` | `3,14,18` | `RQ-K-001/002` |
| `crates/services/src/market/mod.rs` | `20,165,180,188` | `RQ-K-001/003/007` |

链路为：

`sync_kline_request -> latest open time + 1 -> CandleService -> CandleQuery ->`
`CryptoExcAllGateway::candles -> legacy Binance raw mapper -> domain Candle -> shard save`。

K1 只替换其中最底层的 provider protocol 能力；同步边界、finality、领域映射与写入
不得倒灌进 SDK。

## 4. WebSocket finality 是独立协议

```bash
git grep -n -E 'BinanceKlinePayload|@kline_|closed|confirm\(|save_candles' \
  f4acef65caca988b8ee9cd5ef9f1f4dd9d3e1c82 -- \
  crates/services/src/market/binance_websocket.rs
```

结果为 9 行：`39,42,71,72,95,111,155,172,173`，绑定
`RQ-K-005`。这里的 `k.x` 是 WebSocket provider 字段；REST K1 不能伪造同等字段。

## 5. 分表、回测与回填

`rust_quant@f4acef65...` 的 committed 代码表明：

- `DataSyncService::run_sync_data_job` 依次创建既有交易对/周期分表、历史回填和增量回填，
  并使用固定 sleep/循环恢复，绑定 `RQ-K-004`；
- `get_quant_core_sharded_candles_for_backtest` 通过
  `PostgresCandleRepository::quoted_table_name` 读取既有分表，并要求
  `confirm='1'`，绑定 `RQ-K-006`；
- `exchange_candle_to_domain` 把 OHLC/volume 转为 `f64`，并在 `closed=None` 时使用
  `close_time <= now` 推断确认，绑定 `RQ-K-003/007`。

这些调用点由未来 Market/Storage Manifest 迁移；K1 的 forbidden paths 明确阻止 SDK
修改分表或数据库逻辑。

## 6. Research archive 路径

以下冻结搜索用于识别 Binance Vision/月包路径，不能当作 REST K1 消费者：

```bash
git grep -n -i -E \
  'fapi\.binance|binance.*klines|klines.*binance|binance vision|data\.binance' \
  f4acef65caca988b8ee9cd5ef9f1f4dd9d3e1c82 -- '*.rs'
```

结果为 51 行，主要集中在
`crates/rust-quant-cli/src/app/market_cross_exchange_basis_panel*`。其中
`binance_klines.rs` 使用 Binance Vision 月 ZIP、checksum、CSV 连续性和独立
`BinanceKlineAudit`；它绑定 `RQ-K-008`，继续作为 research dataset 路径保留。
搜索中的 `exchangeInfo`、funding、positioning 与 execution error fixture 不是 Kline REST
consumer，不纳入 K1 改动。

## 7. 闭包结论

- SDK 14 行 endpoint/mapper/test 命中已逐文件归类；
- Core REST 主链 7 行与 WebSocket finality 9 行已分别归属；
- 分表、回测和研究月包作为独立后继边界记录，而非被忽略或顺手重写；
- 当前工作树新增文件只能由 K1 测试与 migration-check 证明，不能回填为 legacy 事实；
- 只有 K1 工件提交、Registry 更新为 `created` 后，Market F2B 才能消费该前置。
