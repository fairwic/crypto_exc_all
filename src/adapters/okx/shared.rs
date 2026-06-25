pub(super) fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

pub(super) fn parse_u64_string(value: &str) -> Option<u64> {
    if value.is_empty() {
        None
    } else {
        value.parse::<u64>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn okx_shared_helpers_preserve_empty_value_contract() {
        assert_eq!(non_empty(String::new()), None);
        assert_eq!(
            non_empty("BTC-USDT".to_string()),
            Some("BTC-USDT".to_string())
        );
        assert_eq!(parse_u64_string(""), None);
        assert_eq!(parse_u64_string("1710000000000"), Some(1_710_000_000_000));
        assert_eq!(parse_u64_string("bad"), None);
    }
}
