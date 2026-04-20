use binance_rs::BinanceAccount;

#[tokio::main]
async fn main() -> Result<(), binance_rs::Error> {
    let account = BinanceAccount::from_env()?;
    let balances = account.get_balance().await?;
    println!("balance assets: {}", balances.len());
    Ok(())
}
