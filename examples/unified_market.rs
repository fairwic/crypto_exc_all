use crypto_exc_all::{CryptoSdk, Instrument};

#[tokio::main]
async fn main() -> crypto_exc_all::Result<()> {
    let sdk = CryptoSdk::from_env()?;
    let instrument = Instrument::perp("BTC", "USDT");

    for exchange in sdk.configured_exchanges() {
        let ticker = sdk.market(exchange)?.ticker(&instrument).await?;
        println!(
            "{exchange}: {} last={}",
            ticker.exchange_symbol, ticker.last_price
        );
    }

    Ok(())
}
