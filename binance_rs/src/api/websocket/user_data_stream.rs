use super::BinanceWebsocket;
use crate::error::Error;
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use zeroize::{Zeroize, Zeroizing};

const MAX_PRIVATE_MESSAGE_BYTES: usize = 1024 * 1024;

type BinancePrivateSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Binance USDⓈ-M 用户流连接，显式拥有 listenKey 的创建、续期与关闭生命周期。
///
/// 该会话故意不自动重连：断线后的 snapshot/overlap 恢复必须由 Account owner
/// 重新分配 generation 并完成，SDK 不能隐藏可能丢失的账户事实。
pub struct BinanceUserDataStreamSession {
    websocket: BinanceWebsocket,
    socket: BinancePrivateSocket,
    listen_key: Zeroizing<String>,
}

impl BinanceUserDataStreamSession {
    /// 取得 listenKey 后连接官方私有路径；连接失败会关闭本会话已取得的 key。
    pub async fn open(websocket: BinanceWebsocket) -> Result<Self, Error> {
        let response = websocket.start_user_data_stream().await?;
        let listen_key = response
            .get("listenKey")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .map(Zeroizing::new);
        let Some(listen_key) = listen_key else {
            return Err(Error::WebSocketError(
                "创建用户流成功但响应缺少 listenKey".to_string(),
            ));
        };

        let url = match websocket.private_ws_url(&listen_key) {
            Ok(url) => Zeroizing::new(url),
            Err(error) => {
                let _ = websocket.close_user_data_stream().await;
                return Err(error);
            }
        };
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_PRIVATE_MESSAGE_BYTES))
            .max_frame_size(Some(MAX_PRIVATE_MESSAGE_BYTES));
        let socket = match websocket.connect_bounded_socket(&url, config).await {
            Ok((socket, _)) => socket,
            Err(_error) => {
                let _ = websocket.close_user_data_stream().await;
                // 底层握手错误可能携带完整 URL，而 URL 路径中包含临时 listenKey。
                return Err(Error::WebSocketError("连接 Binance 用户流失败".to_string()));
            }
        };

        Ok(Self {
            websocket,
            socket,
            listen_key,
        })
    }

    /// 读取下一条 provider JSON；响应 provider ping，连接正常关闭返回 `None`。
    pub async fn recv_json(&mut self) -> Result<Option<Value>, Error> {
        loop {
            let message = self.socket.next().await;
            let Some(message) = message else {
                return Ok(None);
            };
            match message.map_err(sanitized_receive_error)? {
                Message::Text(text) => return decode_json(text.as_str()).map(Some),
                Message::Binary(bytes) => {
                    let text = std::str::from_utf8(&bytes).map_err(|error| {
                        Error::WebSocketError(format!("用户流 binary frame 不是 UTF-8: {error}"))
                    })?;
                    return decode_json(text).map(Some);
                }
                Message::Ping(payload) => self
                    .socket
                    .send(Message::Pong(payload))
                    .await
                    .map_err(|error| Error::WebSocketError(error.to_string()))?,
                Message::Close(_) => return Ok(None),
                Message::Pong(_) | Message::Frame(_) => {}
            }
        }
    }

    /// 续期当前 listenKey，并确认账户当前活动 key 未被其他会话替换。
    pub async fn keepalive(&self) -> Result<(), Error> {
        let response = self.websocket.keepalive_user_data_stream().await?;
        if response.get("listenKey").and_then(Value::as_str) == Some(self.listen_key.as_str()) {
            return Ok(());
        }
        Err(Error::WebSocketError(
            "Binance keepalive 未确认当前 listenKey".to_string(),
        ))
    }

    /// 关闭 WebSocket 与 listenKey；即使 WS close 失败也会继续请求 REST 关闭。
    pub async fn close(mut self) -> Result<(), Error> {
        let socket_result = self
            .socket
            .send(Message::Close(None))
            .await
            .map_err(|error| Error::WebSocketError(error.to_string()));
        let rest_result = self
            .websocket
            .close_user_data_stream()
            .await
            .map(|_: Value| ());
        self.listen_key.zeroize();
        socket_result.and(rest_result)
    }
}

fn decode_json(text: &str) -> Result<Value, Error> {
    serde_json::from_str(text).map_err(Error::JsonError)
}

/// 将底层 WebSocket 错误收敛为可运维分类，禁止 URL、listenKey 或代理细节进入日志。
fn sanitized_receive_error(error: tokio_tungstenite::tungstenite::Error) -> Error {
    use std::io::ErrorKind;
    use tokio_tungstenite::tungstenite::Error as WebSocketError;

    let category = match error {
        WebSocketError::ConnectionClosed => "connection_closed",
        WebSocketError::AlreadyClosed => "already_closed",
        WebSocketError::Io(error) => match error.kind() {
            ErrorKind::ConnectionReset => "connection_reset",
            ErrorKind::ConnectionAborted => "connection_aborted",
            ErrorKind::BrokenPipe => "broken_pipe",
            ErrorKind::TimedOut => "timed_out",
            ErrorKind::UnexpectedEof => "unexpected_eof",
            _ => "io",
        },
        WebSocketError::Tls(_) => "tls",
        WebSocketError::Capacity(_) => "capacity",
        WebSocketError::Protocol(_) => "protocol",
        WebSocketError::Utf8(_) => "utf8",
        WebSocketError::WriteBufferFull(_) => "write_buffer_full",
        WebSocketError::AttackAttempt => "attack_attempt",
        _ => "other",
    };
    Error::WebSocketReceiveError { category }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn receive_error_reports_only_safe_transport_category() {
        let error = sanitized_receive_error(tokio_tungstenite::tungstenite::Error::Io(
            io::Error::new(io::ErrorKind::ConnectionReset, "sensitive transport detail"),
        ));

        assert!(matches!(
            error,
            Error::WebSocketReceiveError {
                category: "connection_reset"
            }
        ));
        assert!(!error.to_string().contains("sensitive"));
    }
}
