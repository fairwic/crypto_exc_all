# crypto_exc_all

统一加密货币交易所 SDK facade。外部业务只依赖 `crypto_exc_all`，由根 crate 自动加载不同交易所的 API key，并在内部转成对应的 `okx_rs` / `binance_rs` client。

## 发布结构

本项目按三层发布：

- `okx_rs`: OKX 交易所 SDK crate。
- `binance_rs`: Binance USDⓈ-M Futures SDK crate。
- `crypto_exc_all`: 统一入口 crate，通过版本依赖引用各交易所 SDK。

本地开发使用 `path + version` 依赖；发布时先发布 `okx_rs`、`binance_rs`，再发布 `crypto_exc_all`。

```toml
[dependencies]
crypto_exc_all = "0.1"
```

当前根 crate 的默认 feature 会启用 OKX 和 Binance：

```toml
crypto_exc_all = { version = "0.1", default-features = true }
```

只启用 Binance：

```toml
crypto_exc_all = { version = "0.1", default-features = false, features = ["binance"] }
```

## 环境变量

OKX:

```env
OKX_API_KEY=...
OKX_API_SECRET=...
OKX_PASSPHRASE=...
OKX_SIMULATED_TRADING=1
```

也支持 OKX 模拟盘变量：

```env
OKX_SIMULATED_API_KEY=...
OKX_SIMULATED_API_SECRET=...
OKX_SIMULATED_PASSPHRASE=...
```

Binance:

```env
BINANCE_API_KEY=...
BINANCE_API_SECRET=...
BINANCE_PROXY_URL=socks5h://127.0.0.1:7897
```

`BINANCE_PROXY_URL` 可省略；如果传入 `socks5://`，SDK 会自动规范化成 `socks5h://`。

## 统一调用

```rust
use crypto_exc_all::{CryptoSdk, ExchangeId, Instrument};

#[tokio::main]
async fn main() -> crypto_exc_all::Result<()> {
    let sdk = CryptoSdk::from_env()?;
    let instrument = Instrument::perp("BTC", "USDT");

    let ticker = sdk
        .market(ExchangeId::Binance)?
        .ticker(&instrument)
        .await?;

    println!("{} {}", ticker.exchange_symbol, ticker.last_price);

    Ok(())
}
```

遍历所有已配置交易所：

```rust
use crypto_exc_all::{CryptoSdk, Instrument};

#[tokio::main]
async fn main() -> crypto_exc_all::Result<()> {
    let sdk = CryptoSdk::from_env()?;
    let instrument = Instrument::perp("BTC", "USDT");

    for exchange in sdk.configured_exchanges() {
        let ticker = sdk.market(exchange)?.ticker(&instrument).await?;
        println!("{exchange}: {} {}", ticker.exchange_symbol, ticker.last_price);
    }

    Ok(())
}
```

运行示例：

```bash
cargo run --example unified_market
```

## 当前统一能力

- 自动读取 OKX / Binance 凭证。
- `CryptoSdk::from_env()` / `CryptoSdk::from_config()`。
- `sdk.configured_exchanges()`。
- 统一 `Instrument`，自动映射交易所 symbol：
  - Binance 永续：`BTCUSDT`
  - OKX 永续：`BTC-USDT-SWAP`
- 统一 market ticker：
  - `sdk.market(exchange)?.ticker(&instrument).await`
- 统一 account balances：
  - `sdk.account(exchange)?.balances().await`
- 统一错误入口 `crypto_exc_all::Error`。
- `raw` 逃生口：
  - `crypto_exc_all::raw::okx`
  - `crypto_exc_all::raw::binance`

## 测试

根 crate 包含一个外部调用场景集成测试：测试代码只引入 `crypto_exc_all`，通过 mock HTTP 同时调用 OKX 和 Binance 的统一 ticker 接口。

```bash
cargo test -p crypto_exc_all -- --nocapture
```
