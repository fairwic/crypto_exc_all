use okx::api::api_trait::OkxApiTrait;
use okx::config::Credentials;
use okx::{Error, OkxAsset, OkxClient};
#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    let credentials = Credentials::new("xxx", "xxx", "xxx", "0"); // 初始化客户端
    let client: OkxClient = OkxClient::new(credentials).unwrap();
    //获取asset账户余额
    let balances = OkxAsset::new(client).get_balances(None).await?;
    println!("账户余额: {:?}", balances);

    Ok(())
}
