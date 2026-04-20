# Binance USDⓈ-M Futures SDK

当前 crate 是集合 SDK 中 Binance 交易所的第一阶段实现，结构与 `okx_rs` 对齐：

- `config`: `.env` 与运行时配置
- `client`: HTTP 请求、签名、请求头、响应和错误解析
- `api`: 业务域 API wrapper
- `dto`: 请求/响应 DTO
- `error`: SDK 统一错误类型
- `utils`: HMAC-SHA256 签名、query 构造、时间戳工具

## 环境变量

支持当前仓库 `.env` 中的小写变量，也支持常见大写变量：

```dotenv
binance_api_key=your_api_key
binance_api_secret=your_api_secret

# 或
BINANCE_API_KEY=your_api_key
BINANCE_API_SECRET=your_api_secret
```

可选配置：

```dotenv
BINANCE_API_URL=https://fapi.binance.com
BINANCE_SAPI_API_URL=https://api.binance.com
BINANCE_WEB_API_URL=https://www.binance.com
BINANCE_WS_STREAM_URL=wss://fstream.binance.com
BINANCE_RECV_WINDOW_MS=5000
BINANCE_API_TIMEOUT_MS=5000
BINANCE_PROXY_URL=socks5h://127.0.0.1:7897
```

如果未设置 `BINANCE_PROXY_URL`，SDK 也会兼容读取 `ALL_PROXY` / `all_proxy` / `HTTPS_PROXY` / `https_proxy`。
`socks5://` 会自动归一为 `socks5h://`，避免本地 DNS/直连路径导致 Binance Futures 请求超时。
HTTP REST 和 WebSocket 都支持 `socks5h://` / `socks5://` 代理；WebSocket 当前只对 SOCKS5
代理做内置握手，非 SOCKS 的全局代理会被忽略。

## 已支持接口

- 公共接口：`GET /fapi/v1/time`
- 签名只读接口：`GET /fapi/v2/balance`
- 市场数据：
  - `GET /fapi/v1/exchangeInfo`
  - `GET /fapi/v1/depth`
  - `GET /fapi/v1/klines`
  - `GET /fapi/v1/ticker/24hr`
  - `GET /fapi/v1/fundingRate`
  - `GET /fapi/v1/premiumIndex`
  - `GET /fapi/v1/openInterest`
  - `GET /futures/data/openInterestHist`
  - `GET /futures/data/topLongShortPositionRatio`
  - `GET /futures/data/topLongShortAccountRatio`
  - `GET /futures/data/globalLongShortAccountRatio`
  - `GET /futures/data/takerlongshortRatio`
- 账户/持仓：
  - `GET /fapi/v3/account`
  - `GET /fapi/v3/positionRisk`
  - `GET /fapi/v1/income`
  - `GET /fapi/v1/commissionRate`
  - `GET /fapi/v1/accountConfig`
  - `GET /fapi/v1/symbolConfig`
  - `GET /fapi/v1/rateLimit/order`
  - `GET /fapi/v1/leverageBracket`
  - `GET /fapi/v1/multiAssetsMargin`
  - `GET /fapi/v1/positionSide/dual`
- 交易/订单：
  - `POST /fapi/v1/order`
  - `POST /fapi/v1/batchOrders`
  - `POST /fapi/v1/order/test`
  - `PUT /fapi/v1/order`
  - `PUT /fapi/v1/batchOrders`
  - `DELETE /fapi/v1/order`
  - `DELETE /fapi/v1/batchOrders`
  - `DELETE /fapi/v1/allOpenOrders`
  - `GET /fapi/v1/order`
  - `GET /fapi/v1/openOrder`
  - `GET /fapi/v1/openOrders`
  - `GET /fapi/v1/allOrders`
  - `GET /fapi/v1/userTrades`
  - `POST /fapi/v1/leverage`
  - `POST /fapi/v1/marginType`
  - `POST /fapi/v1/positionSide/dual`
  - `POST /fapi/v1/multiAssetsMargin`
  - `POST /fapi/v1/positionMargin`
- 资产/钱包：
  - `GET /sapi/v1/capital/config/getall`
  - `GET /sapi/v1/asset/wallet/balance`
  - `POST /sapi/v3/asset/getUserAsset`
  - `POST /sapi/v1/asset/get-funding-asset`
  - `POST /sapi/v1/asset/transfer`
  - `GET /sapi/v1/asset/transfer`
  - `GET /sapi/v1/capital/deposit/address`
  - `GET /sapi/v1/capital/deposit/hisrec`
  - `GET /sapi/v1/capital/withdraw/history`
  - `POST /sapi/v1/capital/withdraw/apply`
- 公告：
  - `GET /bapi/composite/v1/public/cms/article/list/query`
- WebSocket 辅助接口：
  - `POST /fapi/v1/listenKey`
  - `PUT /fapi/v1/listenKey`
  - `DELETE /fapi/v1/listenKey`
  - `wss://fstream.binance.com/public/...` URL 构造
  - `wss://fstream.binance.com/market/...` URL 构造
  - `wss://fstream.binance.com/private/...` URL 构造
  - WebSocket JSON 消息接收
  - WebSocket `SUBSCRIBE` / `UNSUBSCRIBE`
  - 基础自动重连与订阅重放
  - WebSocket SOCKS5/SOCKS5h 代理连接
  - public / market / private 多连接分流 hub
  - 连接状态、健康度和基础指标
  - 私有用户数据流 typed event parser：
    `listenKeyExpired`、`MARGIN_CALL`、`ORDER_TRADE_UPDATE`、`TRADE_LITE`、
    `ACCOUNT_UPDATE`、`ACCOUNT_CONFIG_UPDATE`、`STRATEGY_UPDATE`、
    `GRID_UPDATE`、`CONDITIONAL_ORDER_TRIGGER_REJECT`、`ALGO_UPDATE`
  - route URL 已内嵌 `?streams=` / `listenKey=` 时避免重复发送 `SUBSCRIBE`

尚未覆盖的 OKX 侧能力主要是少量非 Futures 专属公共接口。
公告 wrapper 使用 Binance 网站 BAPI，Binance Developer Community 已说明该类 `bapi`
不属于正式公开开发者端点，生产使用时需要按可用性单独监控。

## 示例

```rust
use binance_rs::BinanceAccount;

#[tokio::main]
async fn main() -> Result<(), binance_rs::Error> {
    let account = BinanceAccount::from_env()?;
    let balances = account.get_balance().await?;
    println!("balance assets: {}", balances.len());
    Ok(())
}
```

### 测试下单示例

`test_order` 会走 Binance 的测试下单接口，不会创建真实订单：

```rust
use binance_rs::api::trade::{BinanceTrade, NewOrderRequest};

#[tokio::main]
async fn main() -> Result<(), binance_rs::Error> {
    let trade = BinanceTrade::from_env()?;
    let order = NewOrderRequest::limit("BTCUSDT", "BUY", "0.001", "9000", "GTC");
    trade.test_order(order).await?;
    Ok(())
}
```

运行：

```bash
cargo run --manifest-path binance_rs/Cargo.toml --example account_balance
```

### 资产/钱包示例

SAPI 资产接口使用 Binance 现货主站地址，默认是 `https://api.binance.com`。
如需测试网或代理网关，可通过 `BINANCE_SAPI_API_URL` 覆盖。

```rust
use binance_rs::api::asset::{BinanceAsset, UserAssetRequest};

#[tokio::main]
async fn main() -> Result<(), binance_rs::Error> {
    let asset = BinanceAsset::from_env()?;
    let assets = asset
        .get_user_assets(UserAssetRequest::new().with_asset("USDT"))
        .await?;
    println!("asset rows: {}", assets.len());
    Ok(())
}
```

### 公告示例

```rust
use binance_rs::api::announcements::{AnnouncementListRequest, BinanceAnnouncements};

#[tokio::main]
async fn main() -> Result<(), binance_rs::Error> {
    let api = BinanceAnnouncements::from_env()?;
    let response = api
        .get_announcements(AnnouncementListRequest::latest().with_page_size(20))
        .await?;
    println!("announcements: {}", response);
    Ok(())
}
```

### WebSocket listenKey 和 URL 示例

```rust
use binance_rs::api::websocket::BinanceWebsocket;

#[tokio::main]
async fn main() -> Result<(), binance_rs::Error> {
    let ws = BinanceWebsocket::from_env()?;
    let listen_key = ws.start_user_data_stream().await?["listenKey"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let url = ws.private_ws_url(&listen_key, &["ORDER_TRADE_UPDATE", "ACCOUNT_UPDATE"]);
    println!("private stream url: {}", url);
    ws.close_user_data_stream().await?;
    Ok(())
}
```

运行：

```bash
cargo run --manifest-path binance_rs/Cargo.toml --example user_stream_listen_key
```

公共行情 WebSocket 接收一条消息：

```bash
cargo run --manifest-path binance_rs/Cargo.toml --example websocket_public_stream
```

按 Binance 新分流规则同时连接 public 和 market route：

```bash
cargo run --manifest-path binance_rs/Cargo.toml --example websocket_split_hub
```

可选参数：

```bash
BINANCE_WS_STREAM=btcusdt@aggTrade
BINANCE_WS_STREAM_URL=wss://fstream.binance.com
```

### 实盘 post-only 下单验证

`live_post_only_order` 会读取 `exchangeInfo` 和 24h ticker，计算最小 `LIMIT + GTX`
订单，真实调用 `POST /fapi/v1/order`。如果订单被接受，示例会立即调用
`DELETE /fapi/v1/order` 撤单。

```bash
BINANCE_LIVE_ORDER_CONFIRM=I_UNDERSTAND_THIS_USES_REAL_FUNDS \
cargo run --manifest-path binance_rs/Cargo.toml --example live_post_only_order
```

可选参数：

```bash
BINANCE_LIVE_SYMBOL=BTCUSDT
BINANCE_LIVE_SIDE=BUY
BINANCE_LIVE_POSITION_SIDE=LONG
BINANCE_LIVE_PRICE_OFFSET_BPS=1400
BINANCE_LIVE_SKIP_CANCEL=false
```
