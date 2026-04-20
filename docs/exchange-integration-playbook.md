# Exchange Integration Playbook

这份手册沉淀本轮 `okx`、`bnb_rs`、`bitget_rs`、`crypto_exc_all` 迭代中的经验，用于后续继续接入 Bybit、Hyperliquid 等交易所。

## 当前发布形态

项目采用 monorepo + 多 crate 发布：

```text
crypto_exc_all/
  Cargo.toml              # workspace + 统一 facade crate
  src/                    # 统一 SDK 层
  okx_rs/                 # OKX SDK 源码目录，发布包名 okx
  binance_rs/             # Binance SDK 源码目录，发布包名 bnb_rs
  bitget_rs/              # Bitget SDK 源码目录，发布包名 bitget_rs
```

已经发布到 crates.io：

- `okx = 0.2.1`
- `bnb_rs = 0.1.0`
- `crypto_exc_all = 0.1.0`

当前源码新增但尚未发布：

- `bitget_rs = 0.1.0`
- `crypto_exc_all = 0.1.1` 默认 feature 已接入 `bitget`
- Bitget REST parity 已补到 market/account/trade/asset/announcements/common trade-rate；WebSocket parity 仍作为下一阶段。

根 crate 使用 dependency alias，让外部和内部代码仍然可以用清晰的模块名：

```toml
[dependencies]
binance_rs = { package = "bnb_rs", version = "0.1.0", path = "binance_rs", optional = true }
bitget_rs = { version = "0.1.0", path = "bitget_rs", optional = true }
okx_rs = { package = "okx", version = "0.2.1", path = "okx_rs", optional = true }
```

经验：目录名、Rust crate 导入名、crates.io 包名可以不同。后续如果 crates.io 包名被占用，优先使用 alias，而不是重命名整个源码目录。

## 新增交易所的目标形态

新增交易所时要同时交付两层：

1. 独立交易所 SDK crate
   - 例如 `bybit_rs/` 目录，发布包名可以是 `bybit_rs` 或可用的新名称。
   - 负责该交易所原生 REST/WebSocket、签名、DTO、错误解析、示例和 mock 测试。

2. 根 crate 统一 adapter
   - 在 `crypto_exc_all/src/adapters/` 下增加 adapter。
   - 把统一 `Instrument`、`Ticker`、`Balance`、`Error` 转换到交易所原生 SDK。
   - 外部用户只依赖 `crypto_exc_all`。

外部调用目标保持一致：

```rust
use crypto_exc_all::{CryptoSdk, ExchangeId, Instrument};

#[tokio::main]
async fn main() -> crypto_exc_all::Result<()> {
    let sdk = CryptoSdk::from_env()?;
    let inst = Instrument::perp("BTC", "USDT");
    let ticker = sdk.market(ExchangeId::Bybit)?.ticker(&inst).await?;
    println!("{} {}", ticker.exchange_symbol, ticker.last_price);
    Ok(())
}
```

## 文件清单

新增一个交易所时，通常需要新增或修改：

```text
<exchange>_rs/Cargo.toml
<exchange>_rs/src/lib.rs
<exchange>_rs/src/config.rs
<exchange>_rs/src/client.rs
<exchange>_rs/src/error.rs
<exchange>_rs/src/api/
<exchange>_rs/src/dto/
<exchange>_rs/tests/
<exchange>_rs/examples/

Cargo.toml
src/exchange.rs
src/config.rs
src/error.rs
src/instrument.rs
src/adapters/mod.rs
src/adapters/<exchange>.rs
src/market.rs
src/account.rs
src/lib.rs
tests/external_consumer_tests.rs
README.md
docs/exchange-integration-playbook.md
```

## 接入步骤

### 1. 确定 crates.io 包名

先查包名是否可用：

```bash
cargo search <candidate_name> --limit 5
cargo info <candidate_name>
```

规则：

- 如果官方/通用包名可用，可以直接使用，例如 `bybit_rs`。
- 如果包名已被占用，但我们仍想保留源码目录名，用 dependency alias：

```toml
bybit_rs = { package = "crypto_exc_all_bybit", version = "0.1.0", path = "bybit_rs", optional = true }
```

经验：`okx_rs` 在 crates.io 上实际包名是 `okx`，`binance_rs` 最终发布为 `bnb_rs`。`bitget_rs` 当前保持源码目录名和包名一致。不要把目录名等同于发布包名。

### 2. 新增 workspace member 和 feature

根 `Cargo.toml` 增加：

```toml
[workspace]
members = [
    "okx_rs",
    "binance_rs",
    "bybit_rs",
]

[features]
default = ["okx", "binance", "bybit"]
bybit = ["dep:bybit_rs"]

[dependencies]
bybit_rs = { package = "bybit_rs", version = "0.1.0", path = "bybit_rs", optional = true }
```

经验：根 crate 的 feature 要指向 dependency alias 名，例如 `bybit = ["dep:bybit_rs"]`。

### 3. 实现交易所 SDK 基础层

每个交易所 SDK 至少要有：

- `Config`
  - API base URL
  - WebSocket URL
  - timeout
  - recv window 或 request expiration
  - proxy URL

- `Credentials`
  - API key
  - secret
  - passphrase / memo / subaccount 等交易所特有字段

- `Client`
  - public request
  - signed request
  - API-key-only request
  - timestamp provider for tests
  - status code + exchange error body parsing

- `Error`
  - config
  - http
  - json
  - signature
  - exchange API error
  - websocket
  - missing credentials

经验：不同交易所凭证结构不同，不要在底层强行统一。统一发生在根 crate adapter。

### 4. 签名和请求必须先写测试

先用官方文档样例或 mock server 验证：

- query string 顺序
- body 是否参与签名
- timestamp/recvWindow/expiration
- header 名称
- HTTP method
- path 和 query 拼接
- 错误响应解析

测试应覆盖：

```text
正常响应
HTTP 非 2xx
交易所业务错误 code
缺少凭证
签名固定样例
可选参数为空
可选参数多值
```

经验：签名错通常不是算法错，而是 path、query 顺序、body、timestamp 或 header 名称错。先 mock 固定时间戳，避免测试漂移。

### 5. 先补 OKX parity 的核心域

优先顺序：

1. Market
   - ticker
   - tickers
   - depth/order book
   - klines/candles
   - instruments/exchange info
   - mark price/open interest/funding rate if derivatives

2. Account
   - balances
   - positions
   - account config
   - fee/commission
   - leverage bracket / risk limit

3. Trade
   - place order
   - test order if supported
   - cancel order
   - batch order
   - amend order
   - open orders
   - order history
   - fills/trades

4. Asset
   - currencies
   - deposit address
   - deposit history
   - withdraw history
   - transfer if supported

5. WebSocket
   - public stream
   - private listen key / auth
   - reconnect
   - subscription replay
   - typed user events
   - proxy

经验：先让 REST 核心闭环，再补 WebSocket。WebSocket 稳定性依赖 URL 模式、ping/pong、重连和订阅恢复，不宜只做简单 connect 示例。

Bitget 本轮落地经验：

- Bitget V2 signed POST 使用 JSON body，body 必须参与 `timestamp + METHOD + path + ?query + body` 签名。
- Bitget 官方公告接口路径是 `/api/v2/public/annoucements`，文档和接口都使用这个拼写。
- `productType`、`marginCoin` 等字段大小写在官方文档中不完全一致；SDK 不强制改写用户输入，只在根 adapter 默认传 `USDT-FUTURES`。
- 先用 `serde_json::Value` 承接大量交易所特有响应，等统一 facade 需要稳定字段时再补强 typed DTO。
- 对账、订单、钱包历史这类列表接口要使用 request builder，避免公开方法参数爆炸。

### 6. 根 crate 增加 ExchangeId

`src/exchange.rs` 增加枚举：

```rust
pub enum ExchangeId {
    Okx,
    Binance,
    Bybit,
}
```

同步更新：

- `as_str()`
- `Display`
- `FromStr`
- tests

经验：`ExchangeId` 是根 SDK 的稳定公共 API，命名要短且稳定，不要暴露包名细节，例如用 `Binance`，不用 `BnbRs`。

### 7. 根 crate 增加配置读取

`src/config.rs` 增加交易所配置：

```rust
pub struct BybitExchangeConfig {
    pub api_key: String,
    pub api_secret: String,
    pub api_url: Option<String>,
    pub ws_url: Option<String>,
    pub api_timeout_ms: Option<u64>,
    pub recv_window_ms: Option<u64>,
    pub proxy_url: Option<String>,
}
```

环境变量命名：

```env
BYBIT_API_KEY=...
BYBIT_API_SECRET=...
BYBIT_API_URL=...
BYBIT_WS_URL=...
BYBIT_PROXY_URL=...
```

必须支持：

- `from_env()`
- `from_lookup()` for tests
- `configured_exchanges()`

经验：不要通过 API key 字符串猜交易所。应该通过 env 前缀或显式 config 判断。

### 8. 根 crate 增加 Instrument 映射

在 `src/instrument.rs` 增加交易所 symbol 映射：

```rust
impl Instrument {
    pub fn symbol_for(&self, exchange: ExchangeId) -> String {
        match exchange {
            ExchangeId::Okx => self.okx_symbol(),
            ExchangeId::Binance => self.binance_symbol(),
            ExchangeId::Bybit => self.bybit_symbol(),
        }
    }
}
```

经验：不要让统一接口直接传交易所原始 symbol。统一入口使用 `Instrument::perp("BTC", "USDT")`，adapter 内部映射：

- OKX perpetual: `BTC-USDT-SWAP`
- Binance USD-M perpetual: `BTCUSDT`
- Bybit linear perpetual: usually `BTCUSDT`
- Bitget USDT-FUTURES: often `BTCUSDT`

如果交易所有多产品线，后续可增加：

```rust
InstrumentRef::Raw { exchange, symbol }
```

### 9. 实现 adapter

新增 `src/adapters/bybit.rs`：

```rust
pub(crate) struct BybitAdapter {
    account: BybitAccount,
    market: BybitMarket,
}

impl BybitAdapter {
    pub(crate) fn new(config: BybitExchangeConfig) -> Result<Self> {
        // convert root config into bybit_rs config/client
    }

    pub(crate) async fn ticker(&self, instrument: &Instrument) -> Result<Ticker> {
        // native DTO -> root Ticker
    }

    pub(crate) async fn balances(&self) -> Result<Vec<Balance>> {
        // native DTO -> root Balance
    }
}
```

更新 `src/adapters/mod.rs`：

```rust
#[cfg(feature = "bybit")]
mod bybit;

pub(crate) enum ExchangeClient {
    Okx(OkxAdapter),
    Binance(BinanceAdapter),
    Bybit(BybitAdapter),
}
```

经验：第一版不用 `async_trait` 和 `dyn Trait`。交易所数量有限时，`enum + match` 更清晰，也避免 async trait object 的复杂性。

### 10. 统一 DTO 映射规则

根 DTO 保持小而稳定：

```rust
pub struct Ticker {
    pub exchange: ExchangeId,
    pub instrument: Instrument,
    pub exchange_symbol: String,
    pub last_price: String,
    pub bid_price: Option<String>,
    pub ask_price: Option<String>,
    pub volume_24h: Option<String>,
    pub timestamp: Option<u64>,
    pub raw: serde_json::Value,
}

pub struct Balance {
    pub exchange: ExchangeId,
    pub asset: String,
    pub total: String,
    pub available: String,
    pub frozen: Option<String>,
    pub raw: serde_json::Value,
}
```

规则：

- 统一字段服务 80% 常用场景。
- 交易所特有字段必须保留在 `raw`。
- 金额、价格、数量统一用 `String`，避免精度损失。
- 时间戳优先用毫秒 `u64`，不确定则放 `None` 并保留 `raw`。

经验：不要为了统一 DTO 伪造字段。没有数据就用 `None`，不支持就返回 `Error::Unsupported`。

### 11. 统一错误映射

根错误保留交易所上下文：

```rust
Error::Api {
    exchange,
    status,
    code,
    message,
}
```

adapter 中实现：

```rust
impl Error {
    pub(crate) fn from_bybit(error: bybit_rs::Error) -> Self {
        // map native error into root error
    }
}
```

经验：不要把某个交易所的错误码设计成全局错误码。全局错误必须包含 `exchange` 和原始 `code`。

### 12. 外部调用集成测试

每新增一个交易所，都要更新 `tests/external_consumer_tests.rs`：

- 只 `use crypto_exc_all::*`
- 使用 mock HTTP
- 校验统一接口调用
- 校验交易所实际路径、query、headers
- 校验 unified DTO 映射

目标测试形态：

```rust
let sdk = CryptoSdk::from_config(SdkConfig {
    bybit: Some(BybitExchangeConfig {
        api_key: "key".into(),
        api_secret: "secret".into(),
        api_url: Some(server.url()),
        // ...
    }),
    ..Default::default()
});

let ticker = sdk
    .market(ExchangeId::Bybit)?
    .ticker(&Instrument::perp("BTC", "USDT"))
    .await?;
```

经验：根 crate 的测试要模拟真实外部用户，而不是直接调用 adapter 私有结构。

## 发布流程

发布顺序必须是：

1. 发布新增交易所子 crate。
2. 等 crates.io index 可见。
3. 更新根 crate dependency version。
4. 发布 `crypto_exc_all`。

命令：

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test -p crypto_exc_all -- --nocapture
cargo clippy -p crypto_exc_all --all-targets --no-deps -- -D warnings
```

子 crate dry-run：

```bash
cargo publish -p <package_name> --dry-run --allow-dirty
```

实际发布：

```bash
cargo publish -p <package_name> --allow-dirty --no-verify
```

根 crate 发布：

```bash
cargo publish -p crypto_exc_all --dry-run --allow-dirty
cargo publish -p crypto_exc_all --allow-dirty --no-verify
```

经验：

- crates.io 版本不可覆盖。如果版本已存在，只能 bump patch。
- 子 crate 发布后，根 crate 可能因为索引未同步失败。等待几分钟后重试。
- `cargo info <crate>` 在 workspace 中可能优先显示本地 path，不等于远端确认。发布成功以 `cargo publish` 输出为准。
- 发布前必须检查包内容不含 secret：

```bash
cargo package -p <package> --allow-dirty --no-verify
tar -tzf target/package/<package>-<version>.crate | rg '(^|/)\\.env($|\\.)|(^|/)\\.idea(/|$)' || true
```

## Git 和安全流程

必须确保：

- `.env` 在 `.gitignore`
- `.idea/` 在 `.gitignore`
- `target/` 在 `.gitignore`
- 没有嵌套 `.git`
- 没有真实 API key 进入 git index

检查命令：

```bash
git status --ignored --short | sed -n '1,160p'
git diff --cached --name-only | rg '(^|/)\\.env($|\\.)|(^|/)\\.idea(/|$)|(^|/)target(/|$)|(^|/)\\.git(/|$)' || true
find . -mindepth 2 -maxdepth 3 -name .git -type d -print
```

如果子目录有嵌套 `.git`，要决定是 submodule 还是普通源码目录。本项目采用普通源码目录：

```bash
mv okx_rs/.git /tmp/crypto_exc_all_okx_rs_git_backup_YYYYMMDDHHMMSS
git add okx_rs
```

经验：`git add .` 之前先确认 `.env` 被 ignore。没有初始提交时，`git restore --staged` 可能因没有 `HEAD` 失败，可以用：

```bash
git rm -f --cached .env
```

## WebSocket 经验

本轮 Binance WebSocket 的关键经验：

- Binance USD-M 已迁移到 split routes：`/public`、`/market`、`/private`。
- route URL 已包含 `?streams=` 或 `listenKey=` 时，不要再额外发送 JSON `SUBSCRIBE`。
- 需要区分 combined stream URL 模式和 JSON subscribe 模式。
- manager 应暴露连接状态和健康指标：
  - disconnected
  - connecting
  - connected
  - reconnecting
  - stopped
  - last message time
  - reconnect count
  - last error
- typed event parser 应先解包 `{stream, data}`，再按 `data.e` 分发。
- 大 enum 变体可能触发 Clippy `large_enum_variant`，大事件可以 `Box<T>`。

新增交易所 WebSocket 时，至少要测试：

- public stream 收一条消息
- private auth/listen key
- ping/pong
- reconnect
- subscription replay
- proxy
- typed event fallback raw

## 真实 API 测试经验

真实下单测试只应放在显式 example 或手动命令中，不应进入普通 `cargo test`。

要求：

- 默认不下真实单。
- 需要显式 env 开关。
- 下单优先 post-only / test-order。
- 成功创建订单后必须立即撤单。
- 打印 order id/status/executed quantity，但不打印 API key。

经验：真实交易失败不是单一原因，常见链路包括：

- IP whitelist
- key 权限
- hedge mode 需要 position side
- margin 不足
- symbol 最小下单量
- proxy/DNS 问题

## 常见错误和处理

### 1. crates.io 403 owner 错误

现象：

```text
this crate exists but you don't seem to be an owner
```

处理：

- 确认实际 crates.io 包名。
- 如果是自己的包，确认当前 token 对应账号是 owner。
- 如果包名不是自己的，换包名并使用 dependency alias。

### 2. crate version already exists

现象：

```text
crate <name>@<version> already exists
```

处理：

- bump patch version。
- 更新根 crate dependency version。
- 更新 lockfile。
- 重新测试和发布。

### 3. 根 crate 发布找不到子 crate

现象：

```text
no matching package named `<child>` found
```

处理：

- 先发布子 crate。
- 等 crates.io index 同步。
- 根 crate dependency 使用正确 `package = "..."`
- 重试根 crate publish。

### 4. cargo clippy 检查 path dependency

现象：

```bash
cargo clippy -p crypto_exc_all --all-targets -- -D warnings
```

会把 workspace path dependency 的历史 warning 也拉进来。

处理：

```bash
cargo clippy -p crypto_exc_all --all-targets --no-deps -- -D warnings
```

经验：根 facade 的质量门禁先用 `--no-deps`。如果要全仓库 Clippy，必须单独清理每个子 crate。

## 新增交易所完成定义

一个新交易所算完成，必须满足：

- 子 crate 能 `cargo check`。
- 子 crate 有签名、请求、错误解析测试。
- 子 crate 至少实现 market ticker + account balance。
- 根 crate 有 feature 和 dependency alias。
- 根 crate `ExchangeId`、`SdkConfig`、`Instrument`、adapter 已更新。
- 根 crate 外部调用测试通过。
- README 更新 env 和使用示例。
- 包内容不含 `.env`、`.idea`、`target`。
- 子 crate dry-run publish 通过。
- 根 crate tests 和 clippy 通过。

推荐验证命令：

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test -p crypto_exc_all -- --nocapture
cargo test -p <child_package> -- --nocapture
cargo clippy -p crypto_exc_all --all-targets --no-deps -- -D warnings
cargo publish -p <child_package> --dry-run --allow-dirty
```

## Bitget 首阶段接入记录

Bitget 按同一模式接入：

- 子 crate：`bitget_rs`
- 根 feature：`bitget`
- 默认 product type：`USDT-FUTURES`
- 凭证：`BITGET_API_KEY`、`BITGET_API_SECRET`、`BITGET_PASSPHRASE`
- 公共行情：`GET /api/v2/mix/market/ticker`
- 签名账户：`GET /api/v2/mix/account/accounts`
- 签名头：`ACCESS-KEY`、`ACCESS-SIGN`、`ACCESS-TIMESTAMP`、`ACCESS-PASSPHRASE`
- 根 symbol 映射：`Instrument::perp("BTC", "USDT") -> BTCUSDT`
- 根 ticker 映射：`lastPr`、`bidPr`、`askPr`、`quoteVolume`、`ts`
- 根 balance 映射：`marginCoin`、`accountEquity/usdtEquity`、`available`、`locked`

本轮首阶段只把 Bitget 的只读 market/account 打通到统一 facade。交易、WebSocket、更多 product type、仓位和资产接口按后续 parity 任务扩展。

## 下次新增交易所建议顺序

建议按这个顺序继续：

1. Bybit
2. Hyperliquid

原因：

- Bybit 的 REST/WS 结构更接近 Binance/OKX/Bitget，适合继续复用当前 adapter 模式。
- Hyperliquid 的签名、账户模型和链上语义更特殊，应在统一层稳定后再接。

每次只新增一个交易所，先打通：

```text
config -> client -> market ticker -> account balance -> root adapter -> external consumer test -> dry-run publish
```

再扩展订单、持仓、资产和 WebSocket。
