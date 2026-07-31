#![cfg(all(
    feature = "binance-public-instrument",
    feature = "binance-public-kline",
    feature = "okx-public-market"
))]

use crypto_exc_all::{
    BinancePublicTransportConfig, BinanceUsdmPublicInstrumentClient, BinanceUsdmPublicKlineClient,
    BinanceUsdmPublicKlineConfig, BinanceUsdmPublicKlineQuery, OkxCandleDataset,
    OkxPublicCandleQuery, OkxPublicMarketClient, OkxPublicTransportConfig,
    OkxSwapPublicInstrumentClient,
};
use mockito::Server;
use std::process::Command;

const CHILD_MARKER: &str = "K2_PUBLIC_TRANSPORT_CHILD";
const TEST_NAME: &str = "root_public_facades_use_explicit_transport_without_ambient_env";

/// 同一 provider transport 必须可复用于 Kline/instrument，且 ambient Binance 配置不能污染
/// root public facade。
#[test]
fn root_public_facades_use_explicit_transport_without_ambient_env() {
    if std::env::var_os(CHILD_MARKER).is_some() {
        let runtime = tokio::runtime::Runtime::new().expect("build child runtime");
        runtime.block_on(assert_root_public_transport_contract());
        return;
    }

    // 用子进程注入环境，避免在多线程测试进程内修改 process-global environment。
    let status = Command::new(std::env::current_exe().expect("locate current test binary"))
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD_MARKER, "1")
        .env("BINANCE_API_URL", "http://127.0.0.1:1")
        .env("BINANCE_API_TIMEOUT_MS", "0")
        .env("BINANCE_PROXY_URL", "://ambient-invalid-proxy")
        .status()
        .expect("run isolated ambient-env child");

    assert!(status.success(), "isolated transport contract failed");
}

/// 在隔离子进程中验证四个 root public facade 的真实 HTTP 路径。
async fn assert_root_public_transport_contract() {
    let mut server = Server::new_async().await;
    let binance_kline = server
        .mock("GET", "/fapi/v1/klines?symbol=BTCUSDT&interval=1m")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[[1,"1","2","0.5","1.5","10",59999,"15",4,"6","9","0"]]"#)
        .create_async()
        .await;
    let binance_instrument = server
        .mock("GET", "/fapi/v1/exchangeInfo")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"timezone":"UTC","serverTime":1,"rateLimits":[],"exchangeFilters":[],"assets":[],"symbols":[]}"#,
        )
        .create_async()
        .await;
    let okx_kline = server
        .mock(
            "GET",
            "/api/v5/market/candles?instId=BTC-USDT-SWAP&bar=1m&limit=1",
        )
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[["1","1","2","0.5","1.5","10","1","15","1"]]}"#)
        .create_async()
        .await;
    let okx_instrument = server
        .mock("GET", "/api/v5/public/instruments?instType=SWAP")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"code":"0","msg":"","data":[]}"#)
        .create_async()
        .await;

    let binance_kline_client = BinanceUsdmPublicKlineClient::new(BinanceUsdmPublicKlineConfig {
        api_url: Some(server.url()),
    })
    .expect("legacy-shaped config must map to explicit defaults");
    let binance_transport = BinancePublicTransportConfig {
        api_url: server.url(),
        request_timeout_ms: 500,
        proxy_url: None,
    };
    let binance_instrument_client =
        BinanceUsdmPublicInstrumentClient::with_transport(binance_transport)
            .expect("explicit Binance instrument transport");

    let okx_transport = OkxPublicTransportConfig {
        api_url: server.url(),
        request_timeout_ms: 500,
        proxy_url: None,
    };
    let okx_kline_client = OkxPublicMarketClient::with_transport(okx_transport.clone())
        .expect("explicit OKX Kline transport");
    let okx_instrument_client = OkxSwapPublicInstrumentClient::with_transport(okx_transport)
        .expect("explicit OKX instrument transport");

    binance_kline_client
        .klines(BinanceUsdmPublicKlineQuery::new("BTCUSDT", "1m"))
        .await
        .expect("Binance Kline request");
    binance_instrument_client
        .exchange_info()
        .await
        .expect("Binance instrument request");
    okx_kline_client
        .candles(
            OkxCandleDataset::Recent,
            OkxPublicCandleQuery::new("BTC-USDT-SWAP", "1m").with_limit(1),
        )
        .await
        .expect("OKX Kline request");
    okx_instrument_client
        .instruments()
        .await
        .expect("OKX instrument request");

    binance_kline.assert_async().await;
    binance_instrument.assert_async().await;
    okx_kline.assert_async().await;
    okx_instrument.assert_async().await;
}
