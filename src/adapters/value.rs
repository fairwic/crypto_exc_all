use serde_json::{Map, Value};

pub(crate) fn non_empty_value(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

pub(crate) fn map_string_field(object: &Map<String, Value>, field: &str) -> Option<String> {
    object.get(field).and_then(non_empty_value)
}

pub(crate) fn map_first_string_field(
    object: &Map<String, Value>,
    fields: &[&str],
) -> Option<String> {
    fields
        .iter()
        .find_map(|field| map_string_field(object, field))
}

pub(crate) fn map_u64_field(object: &Map<String, Value>, field: &str) -> Option<u64> {
    object.get(field).and_then(value_as_u64)
}

pub(crate) fn map_first_u64_field(object: &Map<String, Value>, fields: &[&str]) -> Option<u64> {
    fields.iter().find_map(|field| map_u64_field(object, field))
}

pub(crate) fn json_string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(non_empty_value)
}

pub(crate) fn json_first_string_field(value: &Value, fields: &[&str]) -> Option<String> {
    fields
        .iter()
        .find_map(|field| json_string_field(value, field))
}

pub(crate) fn json_u64_field(value: &Value, field: &str) -> Option<u64> {
    value.get(field).and_then(value_as_u64)
}

pub(crate) fn json_first_u64_field(value: &Value, fields: &[&str]) -> Option<u64> {
    fields.iter().find_map(|field| json_u64_field(value, field))
}

pub(crate) fn value_string_at(values: &[Value], index: usize) -> Option<String> {
    values.get(index).and_then(non_empty_value)
}

pub(crate) fn value_u64_at(values: &[Value], index: usize) -> Option<u64> {
    values.get(index).and_then(value_as_u64)
}

pub(crate) fn value_as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(value) => value.as_u64(),
        Value::String(value) => value.parse::<u64>().ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        json_first_string_field, json_first_u64_field, map_first_string_field, map_first_u64_field,
        non_empty_value, value_string_at, value_u64_at,
    };
    use serde_json::json;

    #[test]
    fn non_empty_value_keeps_existing_adapter_semantics() {
        assert_eq!(
            non_empty_value(&json!("BTC-USDT")),
            Some("BTC-USDT".to_string())
        );
        assert_eq!(non_empty_value(&json!(42)), Some("42".to_string()));
        assert_eq!(non_empty_value(&json!(true)), Some("true".to_string()));
        assert_eq!(non_empty_value(&json!("")), None);
        assert_eq!(non_empty_value(&json!(null)), None);
        assert_eq!(non_empty_value(&json!([])), None);
    }

    #[test]
    fn shared_field_helpers_keep_existing_adapter_semantics() {
        let value = json!({
            "symbol": "BTC-USDT",
            "empty": "",
            "id": "42",
            "numeric_id": 7,
            "enabled": true
        });
        let object = value.as_object().expect("test object");

        assert_eq!(
            map_first_string_field(object, &["missing", "symbol"]),
            Some("BTC-USDT".to_string())
        );
        assert_eq!(map_first_string_field(object, &["empty"]), None);
        assert_eq!(
            map_first_string_field(object, &["enabled"]),
            Some("true".to_string())
        );
        assert_eq!(map_first_u64_field(object, &["id"]), Some(42));
        assert_eq!(map_first_u64_field(object, &["numeric_id"]), Some(7));
        assert_eq!(map_first_u64_field(object, &["symbol"]), None);

        assert_eq!(
            json_first_string_field(&value, &["missing", "symbol"]),
            Some("BTC-USDT".to_string())
        );
        assert_eq!(json_first_u64_field(&value, &["id"]), Some(42));
        assert_eq!(value_string_at(&[json!("x")], 0), Some("x".to_string()));
        assert_eq!(value_u64_at(&[json!("42")], 0), Some(42));
    }
}
