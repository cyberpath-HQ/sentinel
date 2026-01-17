//! Comparison utilities for sorting and ordering JSON values.

use serde_json::Value;

/// Compares two JSON values for sorting purposes.
pub fn compare_json_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    fn type_order(v: &Value) -> u8 {
        match v {
            Value::Null => 0,
            Value::Bool(_) => 1,
            Value::Number(_) => 2,
            Value::String(_) => 3,
            Value::Array(_) => 4,
            Value::Object(_) => 5,
        }
    }

    let type_a = type_order(a);
    let type_b = type_order(b);

    if type_a != type_b {
        return type_a.cmp(&type_b);
    }

    match (a, b) {
        (Value::Null, Value::Null) => std::cmp::Ordering::Equal,
        (Value::Bool(ba), Value::Bool(bb)) => ba.cmp(bb),
        (Value::Number(na), Value::Number(nb)) => {
            let fa = na.as_f64().unwrap_or(0.0);
            let fb = nb.as_f64().unwrap_or(0.0);
            fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
        },
        (Value::String(sa), Value::String(sb)) => sa.cmp(sb),
        (Value::Array(aa), Value::Array(ab)) => aa.len().cmp(&ab.len()),
        (Value::Object(oa), Value::Object(ob)) => oa.len().cmp(&ob.len()),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_compare_json_values_null() {
        assert_eq!(
            compare_json_values(&json!(null), &json!(null)),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_json_values(&json!(null), &json!(1)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_json_values(&json!(1), &json!(null)),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_compare_json_values_bool() {
        assert_eq!(
            compare_json_values(&json!(true), &json!(true)),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_json_values(&json!(false), &json!(true)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_json_values(&json!(true), &json!(false)),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_json_values(&json!(true), &json!("string")),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_json_values(&json!("string"), &json!(true)),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_compare_json_values_number() {
        assert_eq!(
            compare_json_values(&json!(1), &json!(1)),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_json_values(&json!(1), &json!(2)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_json_values(&json!(2), &json!(1)),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_json_values(&json!(1.5), &json!(1)),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_json_values(&json!(1), &json!("string")),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_json_values(&json!("string"), &json!(1)),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_compare_json_values_string() {
        assert_eq!(
            compare_json_values(&json!("a"), &json!("a")),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_json_values(&json!("a"), &json!("b")),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_json_values(&json!("b"), &json!("a")),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_json_values(&json!("a"), &json!(1)),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_json_values(&json!(1), &json!("a")),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn test_compare_json_values_array() {
        assert_eq!(
            compare_json_values(&json!([1]), &json!([1])),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_json_values(&json!([1]), &json!([1, 2])),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_json_values(&json!([1, 2]), &json!([1])),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_json_values(&json!([1]), &json!("string")),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_json_values(&json!("string"), &json!([1])),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn test_compare_json_values_object() {
        assert_eq!(
            compare_json_values(&json!({"a":1}), &json!({"a":1})),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_json_values(&json!({"a":1}), &json!({"a":1,"b":2})),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_json_values(&json!({"a":1,"b":2}), &json!({"a":1})),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_json_values(&json!({"a":1}), &json!("string")),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_json_values(&json!("string"), &json!({"a":1})),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn test_compare_values_none() {
        assert_eq!(compare_values(None, None), std::cmp::Ordering::Equal);
        assert_eq!(
            compare_values(None, Some(&json!(1))),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_values(Some(&json!(1)), None),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_compare_values_some() {
        assert_eq!(
            compare_values(Some(&json!(1)), Some(&json!(1))),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_values(Some(&json!(1)), Some(&json!(2))),
            std::cmp::Ordering::Less
        );
    }
}

/// Compares two optional values for sorting purposes.
pub fn compare_values(a: Option<&Value>, b: Option<&Value>) -> std::cmp::Ordering {
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(va), Some(vb)) => compare_json_values(va, vb),
    }
}
