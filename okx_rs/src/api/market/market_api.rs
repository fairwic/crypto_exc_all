use crate::api::api_trait::OkxApiTrait;
use crate::api::API_MARKET_PATH;
#[cfg(feature = "public-market")]
use crate::api::API_PUBLIC_PATH;
use crate::client::OkxClient;
#[cfg(feature = "public-market")]
use crate::client::OkxPublicResponse;
use crate::dto::market::market_dto::{
    CandleOkxRespDto, Depth, InstrumentOkxResDto, TickerOkxResDto,
};
use crate::error::Error;
use log::debug;
use reqwest::Method;
#[cfg(feature = "public-market")]
use serde_json::Value;

/// OKX市场数据API
/// 提供市场行情相关的API访问
#[derive(Debug)]
pub struct OkxMarket {
    /// API客户端
    client: OkxClient,
}

impl OkxApiTrait for OkxMarket {
    /// 创建一个新的OkxMarket实例
    fn new(client: OkxClient) -> Self {
        Self { client }
    }
    /// 从环境变量创建一个新的OkxMarket实例
    #[cfg(feature = "full")]
    fn from_env() -> Result<Self, Error> {
        let client = OkxClient::from_env()?;
        Ok(Self { client })
    }
    /// 获取内部客户端引用
    fn client(&self) -> &OkxClient {
        &self.client
    }
}

impl OkxMarket {
    /// 获取单个 SWAP 的公开标记价，并保留同一次响应的 HTTP/OKX 证据。
    #[cfg(feature = "public-market")]
    pub async fn get_mark_price_with_evidence(
        &self,
        inst_id: &str,
    ) -> Result<OkxPublicResponse<Vec<Value>>, Error> {
        let path = format!(
            "{}/mark-price?instType=SWAP&instId={}",
            API_PUBLIC_PATH, inst_id
        );
        self.client
            .send_public_request_with_evidence::<Vec<Value>>(Method::GET, &path, "")
            .await
    }

    /// 获取单个产品行情信息
    pub async fn get_ticker(&self, inst_id: &str) -> Result<Vec<TickerOkxResDto>, Error> {
        let path = format!("{}/ticker?instId={}", API_MARKET_PATH, inst_id);
        let tickers = self
            .client
            .send_public_request::<Vec<TickerOkxResDto>>(Method::GET, &path, "")
            .await?;
        Ok(tickers)
        // tickers.into_iter().next()
        //     .ok_or_else(|| Error::ParseError("获取行情数据失败: 空响应".to_string()))
    }

    /// 获取多个产品行情信息
    pub async fn get_tickers(&self, inst_type: &str) -> Result<Vec<TickerOkxResDto>, Error> {
        let path = format!("{}/tickers?instType={}", API_MARKET_PATH, inst_type);
        self.client
            .send_public_request::<Vec<TickerOkxResDto>>(Method::GET, &path, "")
            .await
    }

    /// 获取指数行情
    pub async fn get_index_tickers(
        &self,
        quot_ccy: Option<&str>,
        inst_id: Option<&str>,
    ) -> Result<Vec<TickerOkxResDto>, Error> {
        let mut path = format!("{}/index-tickers", API_MARKET_PATH);
        let mut query_params = vec![];

        if let Some(ccy) = quot_ccy {
            query_params.push(format!("quotCcy={}", ccy));
        }

        if let Some(id) = inst_id {
            query_params.push(format!("instId={}", id));
        }

        if !query_params.is_empty() {
            path.push_str(&format!("?{}", query_params.join("&")));
        }

        self.client
            .send_public_request::<Vec<TickerOkxResDto>>(Method::GET, &path, "")
            .await
    }

    /// 获取K线数据。K线数据按请求的粒度分组返回，K线数据每个粒度最多可获取最近1,440条
    /// 限速：40次/2s
    // 限速规则：IP
    pub async fn get_candles(
        &self,
        inst_id: &str,
        bar: &str,
        after: Option<&str>,
        before: Option<&str>,
        limit: Option<&str>,
    ) -> Result<Vec<CandleOkxRespDto>, Error> {
        let path = candle_path("candles", inst_id, bar, after, before, limit);

        let res: Vec<Vec<String>> = self
            .client
            .send_public_request::<Vec<Vec<String>>>(Method::GET, &path, "")
            .await?;
        parse_candle_rows(res)
    }

    /// 获取最近 K 线，并保留同一次 HTTP/OKX envelope 的限频证据。
    #[cfg(feature = "public-market")]
    pub async fn get_candles_with_evidence(
        &self,
        inst_id: &str,
        bar: &str,
        after: Option<&str>,
        before: Option<&str>,
        limit: Option<&str>,
    ) -> Result<OkxPublicResponse<Vec<CandleOkxRespDto>>, Error> {
        let path = candle_path("candles", inst_id, bar, after, before, limit);
        let response = self
            .client
            .send_public_request_with_evidence::<Vec<Vec<String>>>(Method::GET, &path, "")
            .await?;
        parse_candle_response(response)
    }

    // 获取最近几年的历史k线数据(1s k线支持查询最近3个月的数据)
    // 限速：20次/2s
    // 限速规则：IP
    pub async fn get_history_candles(
        &self,
        inst_id: &str,
        bar: &str,
        after: Option<&str>,
        before: Option<&str>,
        limit: Option<&str>,
    ) -> Result<Vec<CandleOkxRespDto>, Error> {
        let path = candle_path("history-candles", inst_id, bar, after, before, limit);
        debug!("OKX path: {}", path);
        let res: Vec<Vec<String>> = self
            .client
            .send_public_request::<Vec<Vec<String>>>(Method::GET, &path, "")
            .await?;
        parse_candle_rows(res)
    }

    /// 获取历史 K 线，并保留同一次 HTTP/OKX envelope 的限频证据。
    #[cfg(feature = "public-market")]
    pub async fn get_history_candles_with_evidence(
        &self,
        inst_id: &str,
        bar: &str,
        after: Option<&str>,
        before: Option<&str>,
        limit: Option<&str>,
    ) -> Result<OkxPublicResponse<Vec<CandleOkxRespDto>>, Error> {
        let path = candle_path("history-candles", inst_id, bar, after, before, limit);
        debug!("OKX path: {}", path);
        let response = self
            .client
            .send_public_request_with_evidence::<Vec<Vec<String>>>(Method::GET, &path, "")
            .await?;
        parse_candle_response(response)
    }

    /// 获取交易产品深度
    pub async fn get_books(&self, inst_id: &str, sz: Option<u32>) -> Result<Depth, Error> {
        let mut path = format!("{}/books?instId={}", API_MARKET_PATH, inst_id);

        if let Some(s) = sz {
            path.push_str(&format!("&sz={}", s));
        }

        let depths = self
            .client
            .send_public_request::<Vec<Depth>>(Method::GET, &path, "")
            .await?;

        depths
            .into_iter()
            .next()
            .ok_or_else(|| Error::ParseError("获取深度数据失败: 空响应".to_string()))
    }

    /// 获取产品列表
    pub async fn get_instruments(
        &self,
        inst_type: &str,
        uly: Option<&str>,
        inst_id: Option<&str>,
    ) -> Result<Vec<InstrumentOkxResDto>, Error> {
        let mut path = format!("{}/instruments?instType={}", API_MARKET_PATH, inst_type);

        if let Some(u) = uly {
            path.push_str(&format!("&uly={}", u));
        }

        if let Some(id) = inst_id {
            path.push_str(&format!("&instId={}", id));
        }

        self.client
            .send_public_request::<Vec<InstrumentOkxResDto>>(Method::GET, &path, "")
            .await
    }
}

/// 生成与 legacy 和 typed evidence 入口共用的 candle 请求路径。
fn candle_path(
    endpoint: &str,
    inst_id: &str,
    bar: &str,
    after: Option<&str>,
    before: Option<&str>,
    limit: Option<&str>,
) -> String {
    let mut path = format!("{API_MARKET_PATH}/{endpoint}?instId={inst_id}&bar={bar}");
    if let Some(after) = after {
        path.push_str(&format!("&after={after}"));
    }
    if let Some(before) = before {
        path.push_str(&format!("&before={before}"));
    }
    if let Some(limit) = limit {
        path.push_str(&format!("&limit={limit}"));
    }
    path
}

/// 映射 candle 行时保留已经取得的同次成功响应证据。
#[cfg(feature = "public-market")]
fn parse_candle_response(
    response: OkxPublicResponse<Vec<Vec<String>>>,
) -> Result<OkxPublicResponse<Vec<CandleOkxRespDto>>, Error> {
    Ok(OkxPublicResponse {
        data: parse_candle_rows(response.data)?,
        evidence: response.evidence,
    })
}

/// 将 provider 行逐行转换为 DTO；短行必须返回定位明确的错误，不能索引 panic。
fn parse_candle_rows(rows: Vec<Vec<String>>) -> Result<Vec<CandleOkxRespDto>, Error> {
    rows.into_iter()
        .enumerate()
        .map(|(index, row)| {
            CandleOkxRespDto::try_from_vec(row)
                .map_err(|message| Error::ParseError(format!("K线第 {index} 行无效: {message}")))
        })
        .collect()
}
#[cfg(all(test, feature = "full"))]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires live OKX credentials and network access"]
    async fn test_get_ticker() {
        let market = OkxMarket::from_env().expect("无法从环境变量创建市场API");
        let ticker = market.get_ticker("BTC-USDT").await;

        println!("Ticker result: {:?}", ticker);
    }

    #[tokio::test]
    #[ignore = "requires live OKX credentials and network access"]
    async fn test_get_candles() {
        let market = OkxMarket::from_env().expect("无法从环境变量创建市场API");
        let candles = market
            .get_candles("BTC-USDT", "1D", None, None, Some("10"))
            .await;

        println!("Candles result: {:?}", candles);
    }
}
