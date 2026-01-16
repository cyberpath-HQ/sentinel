//! Comparison utilities for sorting and ordering JSON values.

use serde_json::Value;

/// Compares two JSON values for sorting purposes.
pub fn compare_json_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (&Value::Null, &Value::Null) => std::cmp::Ordering::Equal,
        (&Value::Null, _) => std::cmp::Ordering::Less,
        (_, &Value::Null) => std::cmp::Ordering::Greater,
        (&Value::Bool(ba), &Value::Bool(bb)) => ba.cmp(&bb),
        (&Value::Bool(_), _) => std::cmp::Ordering::Less,
        (_, &Value::Bool(_)) => std::cmp::Ordering::Greater,
        (&Value::Number(ref na), &Value::Number(ref nb)) => {
            let fa = na.as_f64().unwrap_or(0.0);
            let fb = nb.as_f64().unwrap_or(0.0);
            fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal)
        },
        (&Value::Number(_), _) => std::cmp::Ordering::Less,
        (_, &Value::Number(_)) => std::cmp::Ordering::Greater,
        (&Value::String(ref sa), &Value::String(ref sb)) => sa.cmp(sb),
        (&Value::String(_), _) => std::cmp::Ordering::Less,
        (_, &Value::String(_)) => std::cmp::Ordering::Greater,
        (&Value::Array(ref aa), &Value::Array(ref ab)) => aa.len().cmp(&ab.len()),
        (&Value::Array(_), _) => std::cmp::Ordering::Less,
        (_, &Value::Array(_)) => std::cmp::Ordering::Greater,
        (&Value::Object(ref oa), &Value::Object(ref ob)) => oa.len().cmp(&ob.len()),
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
