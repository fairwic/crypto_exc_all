use bitget_rs::BitgetAccount;
use std::env;

#[tokio::main]
async fn main() -> Result<(), bitget_rs::Error> {
    let product_type =
        env::var("BITGET_PRODUCT_TYPE").unwrap_or_else(|_| "USDT-FUTURES".to_string());
    let account = BitgetAccount::from_env()?;
    let accounts = account.get_accounts(&product_type).await?;
    let coins = accounts
        .iter()
        .map(|account| account.margin_coin.as_str())
        .collect::<Vec<_>>()
        .join(",");

    println!(
        "bitget accounts product_type={} count={} coins={}",
        product_type,
        accounts.len(),
        coins
    );

    Ok(())
}
