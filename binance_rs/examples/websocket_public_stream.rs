use binance_rs::api::websocket::BinanceWebsocket;
use binance_rs::config::Config;
use std::env;
use tokio::time::{Duration, timeout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    let stream_base_url =
        env::var("BINANCE_WS_STREAM_URL").unwrap_or_else(|_| config.ws_stream_url.clone());
    let stream = env::var("BINANCE_WS_STREAM").unwrap_or_else(|_| "btcusdt@aggTrade".to_string());
    let mut websocket = BinanceWebsocket::new_public_with_stream_base_url(stream_base_url);
    if let Some(proxy_url) = config.proxy_url {
        websocket = websocket.with_proxy_url(proxy_url);
    }

    let url = websocket.market_stream_url(&[stream.as_str()]);
    let mut session = websocket.connect_url(&url).await?;

    let message = timeout(Duration::from_secs(10), session.recv_json())
        .await
        .map_err(|_| "timed out waiting for public websocket message")?
        .ok_or("websocket closed before receiving a message")?;

    println!("{}", serde_json::to_string_pretty(&message)?);
    let _ = session.close().await;

    Ok(())
}
