# bitget_rs

Bitget V2 Futures SDK crate used by `crypto_exc_all`.

Current coverage:

- Public common/notice:
  - `GET /api/v2/public/time`
  - `GET /api/v2/public/annoucements`
- Futures market:
  - `GET /api/v2/mix/market/ticker`
  - `GET /api/v2/mix/market/tickers`
  - `GET /api/v2/mix/market/contracts`
  - `GET /api/v2/mix/market/orderbook`
  - `GET /api/v2/mix/market/merge-depth`
  - `GET /api/v2/mix/market/candles`
  - `GET /api/v2/mix/market/history-candles`
  - `GET /api/v2/mix/market/symbol-price`
  - `GET /api/v2/mix/market/current-fund-rate`
  - `GET /api/v2/mix/market/history-fund-rate`
  - `GET /api/v2/mix/market/open-interest`
  - `GET /api/v2/mix/market/oi-limit`
  - `GET /api/v2/mix/market/query-position-lever`
  - `GET /api/v2/mix/market/long-short`
  - `GET /api/v2/mix/market/account-long-short`
  - `GET /api/v2/mix/market/taker-buy-sell`
  - `GET /api/v2/mix/market/exchange-rate`
- Futures account/position:
  - `GET /api/v2/mix/account/accounts`
  - `GET /api/v2/mix/account/account`
  - `GET /api/v2/mix/account/bill`
  - `GET /api/v2/mix/position/all-position`
  - `POST /api/v2/mix/account/set-leverage`
  - `POST /api/v2/mix/account/set-margin-mode`
  - `POST /api/v2/mix/account/set-position-mode`
  - `POST /api/v2/mix/account/set-margin`
  - `POST /api/v2/mix/account/set-asset-mode`
  - `GET /api/v2/common/trade-rate`
- Futures trade:
  - `POST /api/v2/mix/order/place-order`
  - `POST /api/v2/mix/order/batch-place-order`
  - `POST /api/v2/mix/order/cancel-order`
  - `POST /api/v2/mix/order/cancel-batch-orders`
  - `POST /api/v2/mix/order/cancel-all-orders`
  - `POST /api/v2/mix/order/modify-order`
  - `POST /api/v2/mix/order/close-positions`
  - `GET /api/v2/mix/order/detail`
  - `GET /api/v2/mix/order/orders-pending`
  - `GET /api/v2/mix/order/orders-history`
  - `GET /api/v2/mix/order/fills`
- Spot wallet/asset:
  - `GET /api/v2/spot/public/coins`
  - `GET /api/v2/spot/wallet/deposit-address`
  - `GET /api/v2/spot/wallet/deposit-records`
  - `GET /api/v2/spot/wallet/withdrawal-records`
  - `GET /api/v2/spot/wallet/transfer-coin-info`
  - `POST /api/v2/spot/wallet/transfer`
  - `POST /api/v2/spot/wallet/withdrawal`
- HMAC-SHA256 + Base64 request signing
- SOCKS/HTTP proxy configuration
- Mocked request, signature header, and response mapping tests

## Environment

```env
BITGET_API_KEY=...
BITGET_API_SECRET=...
BITGET_PASSPHRASE=...
BITGET_API_URL=https://api.bitget.com
BITGET_API_TIMEOUT_MS=5000
BITGET_PROXY_URL=socks5h://127.0.0.1:7897
```

Lowercase key names are also accepted for compatibility with existing `.env` files:

```env
bitget_api_key=...
bitget_api_secret=...
bitget_passphrase=...
bitget_PASSPHRASE=...
```

## Example

```rust
use bitget_rs::api::market::{BitgetMarket, TickerRequest};

#[tokio::main]
async fn main() -> Result<(), bitget_rs::Error> {
    let market = BitgetMarket::new_public()?;
    let tickers = market
        .get_ticker(TickerRequest::new("BTCUSDT", "USDT-FUTURES"))
        .await?;

    println!("{} {}", tickers[0].symbol, tickers[0].last_price);
    Ok(())
}
```
