# MIG-EXSDK-K1 Binance USD-M 公共 K 线业务语义矩阵

## 1. 结论

K1 只迁移 Binance USD-M REST 公共 K 线的 provider 协议能力，不迁移 Market
分页、finality、Decimal 领域映射、同步调度、分表持久化、WebSocket 或研究月包。
目标不是重写旧流程，而是先把旧流程中应保留、应纠正和应延期的语义逐项归属。

## 2. 旧语义处置

| ID | 冻结事实 | 处置 | K1 目标 / 后继 Owner |
| --- | --- | --- | --- |
| `EXSDK-K-001` | `KlineRequest` 传递 `symbol/interval/startTime/endTime/limit`，调用匿名 `GET /fapi/v1/klines` | 保留 | typed 方法复用同一 request contract，不交换或丢弃边界 |
| `EXSDK-K-002` | legacy endpoint 返回 `serde_json::Value` | 兼容保留 | 新增 `BinanceUsdmKline`；不删除 raw 方法 |
| `EXSDK-K-003` | root mapper 对 OHLC/volume 缺失使用空字符串，对时间缺失使用 `None` | 纠正 | typed row 的 12 个标准位置全部必需，坏行使整批 Decode 失败 |
| `EXSDK-K-004` | root `Candle.closed` 对 REST 固定为 `None` | 保留边界 | K1 仍不推断 finality；Market F2B 根据 BarFinalization contract 决定 |
| `EXSDK-K-005` | legacy root mapper保留 open/close time、OHLC、base/quote volume，却丢弃 trade count、taker-buy volume 和 ignore | 扩充 | K1 保留全部 12 个标准字段，并保留尾部新增字段 |
| `EXSDK-K-006` | legacy decimal 是 JSON string/number 后再转字符串，后续 Core 转 `f64` | 纠正 | K1 使用 `BinanceWireDecimal`，不经过 `f64`；Market 后继映射 Decimal |
| `EXSDK-K-007` | legacy public call 本身不重试、不 sleep | 保留 | SDK 只执行一次请求；Market 拥有 retry/quota/backoff |
| `EXSDK-K-008` | legacy raw错误不保留同次 HTTP quota evidence | 改进 | 复用 I1 的 typed HTTP/provider failure evidence |
| `EXSDK-K-009` | legacy `binance` root feature 同时开启 full SDK | 兼容保留 | 新增最小 `binance-public-kline`；旧 `binance` 继续包含 full SDK |
| `EXSDK-K-010` | Core 为公共读取构造 dummy 或环境 credential 的 full gateway | 不迁入 | K1 client 永远无凭证；Market F2B 不回退到用户 credential |
| `RQ-K-001` | `CandleService` 将领域边界映射到 `CandleQuery`，再调用 root SDK | 延期迁移 | Market F2B 消费 K1 typed facade，并拥有 query/domain 映射 |
| `RQ-K-002` | 增量同步从最新 open time `checked_add(1)` 形成 start boundary | 记录后重审 | F2B 明确 inclusive/exclusive contract；K1 只忠实传递毫秒值 |
| `RQ-K-003` | legacy Core 依据 `close_time <= now` 推断 REST 棒确认状态 | 禁止复制 | finality 必须由 Market `BarFinalizationV1` 和 observation time 决定 |
| `RQ-K-004` | `DataSyncService` 拥有循环、固定 sleep、回填上限、错误后继续与分表写入 | 延期迁移 | Market/Storage 后继分别拥有调度恢复与既有分表；不下沉 SDK |
| `RQ-K-005` | Binance WebSocket `k.x` 明确表达当前推送是否闭合 | 独立保留 | WebSocket 是另一协议路径，不能用 REST close time伪造 `x` |
| `RQ-K-006` | backtest 只读取 `confirm='1'` 的既有 K 线分表 | 独立保留 | K1 不建表、不读库；Storage 迁移继续复用受治理分表 |
| `RQ-K-007` | legacy Core 把价格和成交量解析为 `f64` | 禁止复制 | Market canonical candle 使用 Decimal；策略计算是否转浮点由独立 contract 决定 |
| `RQ-K-008` | Research 直接使用 Binance Vision 月包、checksum、CSV 连续性与独立审计 | 独立保留 | archive dataset 不由 REST K1 替代，需单独 Dataset Manifest/cutover |

## 3. Provider row 冻结

Binance USD-M K 线数组的标准位置按 provider wire contract 冻结：

| 位置 | 字段 | K1 类型 / 单位 | 业务边界 |
| ---: | --- | --- | --- |
| 0 | open time | `u64`，Unix 毫秒 | 不在 SDK 转时区 |
| 1-4 | OHLC | `BinanceWireDecimal` | 不转 `f64`、不量化 |
| 5 | base asset volume | `BinanceWireDecimal` | 不冒充 quote volume |
| 6 | close time | `u64`，Unix 毫秒 | 不单独证明 final |
| 7 | quote asset volume | `BinanceWireDecimal` | 单位保持 provider 语义 |
| 8 | number of trades | `u64` | 不作为完整性代理 |
| 9 | taker buy base volume | `BinanceWireDecimal` | 原样保留 |
| 10 | taker buy quote volume | `BinanceWireDecimal` | 原样保留 |
| 11 | ignore | `serde_json::Value` | 无业务含义，只保留 |
| 12+ | provider extension | `Vec<Value>` | 前向保留，不猜字段 |

## 4. 查询与分页边界

- K1 只验证 `symbol/interval` 非空且无空白、`limit > 0`；
- K1 不硬编码当前存在歧义的最大 `limit`，也不静默 clamp；
- K1 不判断 `startTime/endTime` 的领域包含关系，不交换边界；
- Market F2B 必须冻结 operational page cap、下一页 cursor、重复 open time 去重、
  空页/短页终止、限频预算与恢复策略；
- K1 返回顺序就是 provider 顺序，不排序、不补洞、不把空数组解释为完整快照。

## 5. 明确不属于 K1 的完整逻辑

- REST 当前棒的 provisional/final 判定；
- WebSocket `k.x`、断线重连与 REST gap repair；
- canonical symbol、timeframe 与 `ConfirmedCandle<Decimal>`；
- 历史/增量回填、幂等、cursor、run ledger、lease、quota、retry；
- 既有生产分表 DDL、schema-tool、回填和 retention；
- Binance Vision research archive、checksum 与 dataset completeness；
- OKX 和其他交易所。

这些不是被丢弃，而是已登记为后继 Owner 的迁移输入。
