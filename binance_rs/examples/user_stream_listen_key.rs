use binance_rs::api::websocket::BinanceWebsocket;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let websocket = BinanceWebsocket::from_env()?;
    let started = websocket.start_user_data_stream().await?;
    let listen_key = started
        .get("listenKey")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    println!("started listenKey: {}", mask_listen_key(listen_key));

    let closed = websocket.close_user_data_stream().await?;
    println!("closed listenKey stream: {}", closed);

    Ok(())
}

fn mask_listen_key(value: &str) -> String {
    if value.len() <= 12 {
        return "***".to_string();
    }

    format!("{}...{}", &value[..6], &value[value.len() - 6..])
}
