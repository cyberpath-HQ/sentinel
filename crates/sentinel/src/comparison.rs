//! Comparison utilities for sorting and ordering JSON values.

use serde_json::Value;

/// Compares two JSON values for sorting purposes.
pub fn compare_json_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    const fn type_order(v: &Value) -> u8 {
        match *v {
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

    #[allow(
        clippy::needless_borrowed_reference,
        reason = "clippy suggestions are incorrect for matching &Value patterns"
    )]
    match (a, b) {
        (&Value::Null, &Value::Null) => std::cmp::Ordering::Equal,
        (&Value::Bool(ba), &Value::Bool(bb)) => ba.cmp(&bb),
        (&Value::Number(ref na), &Value::Number(ref nb)) => {
            match (na.as_f64(), nb.as_f64()) {
                (Some(fa), Some(fb)) => fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal),
                _ => {
                    let sa = na.to_string();
                    let sb = nb.to_string();
                    match (sa.starts_with('-'), sb.starts_with('-')) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => {
                            let (sa_num, sb_num, negative) =
                                sa.strip_prefix('-')
                                    .map_or((sa.as_str(), sb.as_str(), false), |sa_stripped| {
                                        (
                                            sa_stripped,
                                            sb.strip_prefix('-').unwrap_or(sb.as_str()),
                                            true,
                                        )
                                    });
                            let len_cmp = sa_num.len().cmp(&sb_num.len());
                            if len_cmp == std::cmp::Ordering::Equal {
                                let cmp = sa_num.cmp(sb_num);
                                if negative {
                                    cmp.reverse()
                                }
                                else {
                                    cmp
                                }
                            }
                            else if negative {
                                len_cmp.reverse()
                            }
                            else {
                                len_cmp
                            }
                        },
                    }
                },
            }
        },
        (&Value::String(ref sa), &Value::String(ref sb)) => sa.cmp(sb),
        (&Value::Array(ref aa), &Value::Array(ref ab)) => aa.len().cmp(&ab.len()),
        (&Value::Object(ref oa), &Value::Object(ref ob)) => oa.len().cmp(&ob.len()),
        #[cfg(not(tarpaulin_include))]
_ => std::cmp::Ordering::Equal,
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
    fn test_compare_json_values_very_large_numbers() {
        let large1: Value =
            serde_json::from_str("1000000000000000000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();
        let large2: Value =
            serde_json::from_str("2000000000000000000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();
        assert_eq!(
            compare_json_values(&large1, &large2),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_json_values(&large2, &large1),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_json_values(&large1, &large1),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn test_compare_json_values_negative_large_numbers() {
        let neg_large1: Value =
            serde_json::from_str("-1000000000000000000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();
        let neg_large2: Value =
            serde_json::from_str("-2000000000000000000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();
        let pos_large: Value =
            serde_json::from_str("1000000000000000000000000000000000000000000000000000000000000000000000000000000")
                .unwrap();
        assert_eq!(
            compare_json_values(&neg_large1, &neg_large2),
            std::cmp::Ordering::Greater // -100 > -200
        );
        assert_eq!(
            compare_json_values(&neg_large2, &neg_large1),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_json_values(&neg_large1, &pos_large),
            std::cmp::Ordering::Less // negative < positive
        );
        assert_eq!(
            compare_json_values(&pos_large, &neg_large1),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_compare_json_values_large_numbers() {
        let neg_large: Value = json!(-1e308);
        let pos_large: Value = json!(1e308);
        assert_eq!(
            compare_json_values(&neg_large, &pos_large),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_json_values(&pos_large, &neg_large),
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

    #[test]
    fn test_compare_json_values_string_numbers_negative_vs_positive() {
        // Test negative vs positive (line 39: true, false)
        // Use numbers that exceed f64 range (f64::MAX ≈ 1.8e308)
        let neg: Value = serde_json::from_str("-10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let pos: Value = serde_json::from_str("10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();

        assert_eq!(compare_json_values(&neg, &pos), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_compare_json_values_string_numbers_positive_vs_negative() {
        // Test positive vs negative (line 40: false, true)
        let pos: Value = serde_json::from_str("10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let neg: Value = serde_json::from_str("-10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();

        assert_eq!(compare_json_values(&pos, &neg), std::cmp::Ordering::Greater);
    }

    #[test]
    fn test_compare_json_values_both_negative_same_length() {
        // Test both negative, same length, should compare lexicographically and reverse (line 54-58)
        let neg1: Value = serde_json::from_str("-10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let neg2: Value = serde_json::from_str("-90000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();

        // For negative numbers: -1000... > -9000... (smaller absolute value is greater)
        assert_eq!(
            compare_json_values(&neg1, &neg2),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_compare_json_values_both_positive_same_length() {
        // Test both positive, same length, should compare lexicographically (line 58 else branch)
        let pos1: Value = serde_json::from_str("10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let pos2: Value = serde_json::from_str("90000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();

        // For positive numbers: 1000... < 9000... (normal string comparison)
        assert_eq!(compare_json_values(&pos1, &pos2), std::cmp::Ordering::Less);
    }

    #[test]
    fn test_compare_json_values_both_negative_different_length() {
        // Test both negative, different lengths (line 61-63)
        let neg_short: Value = serde_json::from_str("-1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let neg_long: Value = serde_json::from_str("-10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();

        // Shorter negative number is greater (smaller absolute value, closer to zero)
        assert_eq!(
            compare_json_values(&neg_short, &neg_long),
            std::cmp::Ordering::Greater
        );
    }

    #[test]
    fn test_compare_json_values_both_positive_different_length() {
        // Test both positive, different lengths (line 65)
        let pos_short: Value = serde_json::from_str("1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();
        let pos_long: Value = serde_json::from_str("10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();

        // Shorter positive number is less (smaller value)
        assert_eq!(
            compare_json_values(&pos_short, &pos_long),
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
