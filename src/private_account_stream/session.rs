use crate::adapters::ExchangeClient;
use crate::{Error, ExchangeId, Result};
use serde_json::Value;

/// 私有流保活结果；区分 OKX 应用层 heartbeat 与 Binance listenKey 续期。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivateAccountStreamKeepalive {
    /// OKX 已返回对应应用层 heartbeat 的 `pong`。
    HeartbeatConfirmed,
    /// Binance listenKey 已通过 REST keepalive 续期。
    Renewed,
}

/// 统一 SDK 上的私有账户流入口，只开放当前已实现的 OKX/Binance 能力。
pub struct PrivateAccountStreamFacade<'a> {
    client: &'a ExchangeClient,
}

impl<'a> PrivateAccountStreamFacade<'a> {
    pub(crate) const fn new(client: &'a ExchangeClient) -> Self {
        Self { client }
    }

    /// 建立 provider 会话；不自动重连，避免 SDK 隐藏账户事实缺口。
    pub async fn open(&self) -> Result<PrivateAccountStreamSession> {
        self.client.open_private_account_stream().await
    }
}

/// 已建立的统一私有账户流会话。
pub struct PrivateAccountStreamSession {
    exchange: ExchangeId,
    inner: PrivateAccountStreamSessionInner,
}

enum PrivateAccountStreamSessionInner {
    #[cfg(feature = "okx")]
    Okx(Box<okx_rs::websocket::OkxPrivateAccountStreamSession>),
    #[cfg(feature = "binance")]
    Binance(Box<binance_rs::api::websocket::BinanceUserDataStreamSession>),
}

impl PrivateAccountStreamSession {
    #[cfg(feature = "okx")]
    pub(crate) fn from_okx(session: okx_rs::websocket::OkxPrivateAccountStreamSession) -> Self {
        Self {
            exchange: ExchangeId::Okx,
            inner: PrivateAccountStreamSessionInner::Okx(Box::new(session)),
        }
    }

    #[cfg(feature = "binance")]
    pub(crate) fn from_binance(
        session: binance_rs::api::websocket::BinanceUserDataStreamSession,
    ) -> Self {
        Self {
            exchange: ExchangeId::Binance,
            inner: PrivateAccountStreamSessionInner::Binance(Box::new(session)),
        }
    }

    /// 返回该会话固定绑定的交易所。
    pub const fn exchange(&self) -> ExchangeId {
        self.exchange
    }

    /// 读取下一条 provider JSON；断开返回 `None`，不把断线伪装成空业务事件。
    pub async fn recv_json(&mut self) -> Result<Option<Value>> {
        match &mut self.inner {
            #[cfg(feature = "okx")]
            PrivateAccountStreamSessionInner::Okx(session) => {
                session.recv_json().await.map_err(Error::from_okx)
            }
            #[cfg(feature = "binance")]
            PrivateAccountStreamSessionInner::Binance(session) => {
                session.recv_json().await.map_err(Error::from_binance)
            }
        }
    }

    /// 执行 provider 保活：OKX 确认 `pong`，Binance 续期 listenKey。
    pub async fn keepalive(&mut self) -> Result<PrivateAccountStreamKeepalive> {
        match &mut self.inner {
            #[cfg(feature = "okx")]
            PrivateAccountStreamSessionInner::Okx(session) => {
                session.heartbeat().await.map_err(Error::from_okx)?;
                Ok(PrivateAccountStreamKeepalive::HeartbeatConfirmed)
            }
            #[cfg(feature = "binance")]
            PrivateAccountStreamSessionInner::Binance(session) => {
                session.keepalive().await.map_err(Error::from_binance)?;
                Ok(PrivateAccountStreamKeepalive::Renewed)
            }
        }
    }

    /// 主动关闭 provider 会话与其显式生命周期资源。
    pub async fn close(self) -> Result<()> {
        match self.inner {
            #[cfg(feature = "okx")]
            PrivateAccountStreamSessionInner::Okx(session) => {
                session.close().await.map_err(Error::from_okx)
            }
            #[cfg(feature = "binance")]
            PrivateAccountStreamSessionInner::Binance(session) => {
                session.close().await.map_err(Error::from_binance)
            }
        }
    }
}
