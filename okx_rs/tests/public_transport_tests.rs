use okx::{Error, OkxClient, OkxPublicTransportConfig};
use reqwest::Method;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

/// 显式 timeout 必须约束真实 public request，且 SDK 不得在内部重试。
#[tokio::test]
async fn explicit_public_transport_applies_request_timeout() {
    let api_url = delayed_json_server(Duration::from_millis(150));
    let client = OkxClient::new_public_with_transport(OkxPublicTransportConfig {
        api_url,
        request_timeout_ms: 20,
        proxy_url: None,
    })
    .expect("explicit OKX public transport");

    let error = client
        .send_public_request::<Value>(Method::GET, "/slow", "")
        .await
        .expect_err("request must time out");

    let Error::HttpError(source) = error else {
        panic!("expected OKX HTTP timeout");
    };
    assert!(source.is_timeout());
}

/// 配置错误必须在网络 I/O 前失败，且 Debug/错误不得泄漏代理认证信息。
#[test]
fn public_transport_rejects_invalid_values_and_redacts_proxy() {
    let defaults = OkxPublicTransportConfig::default();
    assert_eq!(defaults.api_url, "https://www.okx.com");
    assert_eq!(defaults.request_timeout_ms, 5_000);
    assert_eq!(defaults.proxy_url, None);

    let config = OkxPublicTransportConfig {
        api_url: "https://www.okx.com".to_string(),
        request_timeout_ms: 5_000,
        proxy_url: Some("http://proxy-user:proxy-secret@127.0.0.1:8080".to_string()),
    };
    let debug = format!("{config:?}");
    assert!(debug.contains("proxy_configured"));
    assert!(!debug.contains("proxy-user"));
    assert!(!debug.contains("proxy-secret"));

    let unsafe_endpoint_debug = format!(
        "{:?}",
        OkxPublicTransportConfig {
            api_url: "https://endpoint-user:endpoint-secret@www.okx.com?token=query-secret"
                .to_string(),
            request_timeout_ms: 5_000,
            proxy_url: None,
        }
    );
    assert!(!unsafe_endpoint_debug.contains("endpoint-secret"));
    assert!(!unsafe_endpoint_debug.contains("query-secret"));

    for api_url in [
        "ftp://www.okx.com",
        "https://endpoint-user:endpoint-secret@www.okx.com",
        "https://www.okx.com?region=test",
        "https://www.okx.com#fragment",
    ] {
        let error = OkxClient::new_public_with_transport(OkxPublicTransportConfig {
            api_url: api_url.to_string(),
            request_timeout_ms: 5_000,
            proxy_url: None,
        })
        .expect_err("unsafe endpoint must fail");
        assert!(matches!(error, Error::ConfigError(_)));
        assert!(!error.to_string().contains("endpoint-secret"));
    }

    let error = OkxClient::new_public_with_transport(OkxPublicTransportConfig {
        api_url: "https://www.okx.com".to_string(),
        request_timeout_ms: 0,
        proxy_url: None,
    })
    .expect_err("zero timeout must fail");
    assert!(matches!(error, Error::ConfigError(_)));

    let error = OkxClient::new_public_with_transport(OkxPublicTransportConfig {
        api_url: "https://www.okx.com".to_string(),
        request_timeout_ms: 5_000,
        proxy_url: Some("://invalid-proxy".to_string()),
    })
    .expect_err("invalid proxy must fail");
    let message = error.to_string();
    assert!(!message.contains("invalid-proxy"));
}

/// 启动一个只接受一次请求、延迟返回空 JSON 的本地 HTTP server。
fn delayed_json_server(delay: Duration) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind delayed HTTP server");
    let address = listener.local_addr().expect("read delayed HTTP address");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept delayed HTTP request");
        let mut request = [0_u8; 1_024];
        let _ = stream.read(&mut request);
        thread::sleep(delay);
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}",
        );
    });
    format!("http://{address}")
}
