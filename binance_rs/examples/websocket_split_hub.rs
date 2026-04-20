use binance_rs::api::websocket::{
    BinanceStreamRoute, BinanceWebsocket, BinanceWebsocketEvent, BinanceWebsocketHub,
    StreamSubscription,
};
use binance_rs::config::{Config, DEFAULT_WS_STREAM_URL};
use std::env;
use tokio::time::{Duration, timeout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    let stream_base_url =
        env::var("BINANCE_WS_STREAM_URL").unwrap_or_else(|_| DEFAULT_WS_STREAM_URL.to_string());
    let websocket = BinanceWebsocket::new_public_with_stream_base_url(stream_base_url);

    let mut hub = BinanceWebsocketHub::new()
        .with_route_url(
            BinanceStreamRoute::Public,
            websocket.public_stream_url(&["btcusdt@depth5@100ms"]),
        )
        .with_route_url(
            BinanceStreamRoute::Market,
            websocket.market_stream_url(&["btcusdt@aggTrade"]),
        );

    if let Some(proxy_url) = config.proxy_url {
        hub = hub.with_proxy_url(proxy_url);
    }

    let mut receiver = hub
        .start(vec![
            StreamSubscription::public("btcusdt@depth5@100ms"),
            StreamSubscription::market("btcusdt@aggTrade"),
        ])
        .await?;

    for _ in 0..2 {
        let message = timeout(Duration::from_secs(10), receiver.recv())
            .await
            .map_err(|_| "timed out waiting for websocket hub message")?
            .ok_or("websocket hub closed before receiving a message")?;
        println!("{:#?}", BinanceWebsocketEvent::parse(message)?);
    }

    Ok(())
}
