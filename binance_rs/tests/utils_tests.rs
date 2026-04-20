use binance_rs::utils::{build_query_string, generate_signature};

#[test]
fn generates_hmac_signature_from_binance_documentation_example() {
    let secret = "2b5eb11e18796d12d88f13dc27dbbd02c2cc51ff7059765ed9821957d82bb4d9";
    let payload = "symbol=BTCUSDT&side=BUY&type=LIMIT&quantity=1&price=9000&timeInForce=GTC&recvWindow=5000&timestamp=1591702613943";

    let signature = generate_signature(secret, payload).expect("signature should be generated");

    assert_eq!(
        signature,
        "3c661234138461fcc7a7d8746c6558c9842d4e10870d2ecbedf7777cad694af9"
    );
}

#[test]
fn builds_query_string_in_input_order_without_trailing_separator() {
    let params = [
        ("symbol", "BTCUSDT"),
        ("recvWindow", "5000"),
        ("timestamp", "1591702613943"),
    ];

    let query = build_query_string(&params);

    assert_eq!(
        query,
        "symbol=BTCUSDT&recvWindow=5000&timestamp=1591702613943"
    );
}
