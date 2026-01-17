//! Filtering utilities for document matching.

use serde_json::Value;

use crate::{Document, Filter};

/// Checks if a document matches all the given filters.
pub fn matches_filters(doc: &Document, filters: &[Filter]) -> bool {
    for filter in filters {
        let matches = match filter {
            Filter::Equals(field, value) => doc.data().get(field) == Some(value),
            Filter::GreaterThan(field, value) => {
                match (doc.data().get(field), value) {
                    (Some(Value::Number(n)), Value::Number(v)) => n.as_f64().unwrap_or(0.0) > v.as_f64().unwrap_or(0.0),
                    _ => false,
                }
            },
            Filter::LessThan(field, value) => {
                match (doc.data().get(field), value) {
                    (Some(Value::Number(n)), Value::Number(v)) => n.as_f64().unwrap_or(0.0) < v.as_f64().unwrap_or(0.0),
                    _ => false,
                }
            },
            Filter::GreaterOrEqual(field, value) => {
                match (doc.data().get(field), value) {
                    (Some(Value::Number(n)), Value::Number(v)) => {
                        n.as_f64().unwrap_or(0.0) >= v.as_f64().unwrap_or(0.0)
                    },
                    _ => false,
                }
            },
            Filter::LessOrEqual(field, value) => {
                match (doc.data().get(field), value) {
                    (Some(Value::Number(n)), Value::Number(v)) => {
                        n.as_f64().unwrap_or(0.0) <= v.as_f64().unwrap_or(0.0)
                    },
                    _ => false,
                }
            },
            Filter::In(field, values) => doc.data().get(field).is_some_and(|v| values.contains(v)),
            Filter::Contains(field, substring) => {
                match doc.data().get(field) {
                    Some(Value::Array(arr)) => {
                        arr.iter().any(|v| {
                            if let Value::String(s) = v {
                                s.contains(substring)
                            }
                            else {
                                false
                            }
                        })
                    },
                    Some(Value::String(s)) => s.contains(substring),
                    _ => false,
                }
            },
            Filter::StartsWith(field, prefix) => {
                match doc.data().get(field) {
                    Some(Value::String(s)) => s.starts_with(prefix),
                    _ => false,
                }
            },
            Filter::EndsWith(field, suffix) => {
                match doc.data().get(field) {
                    Some(Value::String(s)) => s.ends_with(suffix),
                    _ => false,
                }
            },
            Filter::Exists(field, exists) => {
                let field_exists = doc.data().get(field).is_some();
                field_exists == *exists
            },
            Filter::And(left, right) => {
                matches_filters(doc, &[*left.clone()]) && matches_filters(doc, &[*right.clone()])
            },
            Filter::Or(left, right) => {
                matches_filters(doc, &[*left.clone()]) || matches_filters(doc, &[*right.clone()])
            },
        };
        if !matches {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{Document, Filter};

    fn create_doc(data: Value) -> Document { Document::new_without_signature("test".to_string(), data).unwrap() }

    #[test]
    fn test_matches_filters_equals() {
        let doc = create_doc(json!({"name": "Alice", "age": 25}));
        let filter = Filter::Equals("name".to_string(), json!("Alice"));
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::Equals("name".to_string(), json!("Bob"));
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_greater_than() {
        let doc = create_doc(json!({"age": 25}));
        let filter = Filter::GreaterThan("age".to_string(), json!(20));
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::GreaterThan("age".to_string(), json!(30));
        assert!(!matches_filters(&doc, &[filter]));

        // Test with non-number field
        let doc = create_doc(json!({"name": "Alice"}));
        let filter = Filter::GreaterThan("name".to_string(), json!(20));
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_less_than() {
        let doc = create_doc(json!({"age": 25}));
        let filter = Filter::LessThan("age".to_string(), json!(30));
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::LessThan("age".to_string(), json!(20));
        assert!(!matches_filters(&doc, &[filter]));

        // Test with non-number field
        let doc = create_doc(json!({"name": "Alice"}));
        let filter = Filter::LessThan("name".to_string(), json!(20));
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_greater_or_equal() {
        let doc = create_doc(json!({"age": 25}));
        let filter = Filter::GreaterOrEqual("age".to_string(), json!(25));
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::GreaterOrEqual("age".to_string(), json!(20));
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::GreaterOrEqual("age".to_string(), json!(30));
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_greater_or_equal_non_number() {
        // Test with non-number field
        let doc = create_doc(json!({"name": "Alice"}));
        let filter = Filter::GreaterOrEqual("name".to_string(), json!(20));
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_less_or_equal() {
        let doc = create_doc(json!({"age": 25}));
        let filter = Filter::LessOrEqual("age".to_string(), json!(25));
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::LessOrEqual("age".to_string(), json!(30));
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::LessOrEqual("age".to_string(), json!(20));
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_less_or_equal_non_number() {
        // Test with non-number field
        let doc = create_doc(json!({"name": "Alice"}));
        let filter = Filter::LessOrEqual("name".to_string(), json!(20));
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_in() {
        let doc = create_doc(json!({"status": "active"}));
        let filter = Filter::In(
            "status".to_string(),
            vec![json!("active"), json!("inactive")],
        );
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::In("status".to_string(), vec![json!("inactive")]);
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_contains_string() {
        let doc = create_doc(json!({"name": "Alice"}));
        let filter = Filter::Contains("name".to_string(), "Ali".to_string());
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::Contains("name".to_string(), "Bob".to_string());
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_contains_array() {
        let doc = create_doc(json!({"tags": ["rust", "programming"]}));
        let filter = Filter::Contains("tags".to_string(), "rust".to_string());
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::Contains("tags".to_string(), "python".to_string());
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_contains_array_mixed_types() {
        // Test array with mixed types - should only match strings
        let doc = create_doc(json!({"tags": ["rust", 42, true]}));
        let filter = Filter::Contains("tags".to_string(), "rust".to_string());
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::Contains("tags".to_string(), "42".to_string());
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_starts_with() {
        let doc = create_doc(json!({"name": "Alice"}));
        let filter = Filter::StartsWith("name".to_string(), "Ali".to_string());
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::StartsWith("name".to_string(), "Bob".to_string());
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_ends_with() {
        let doc = create_doc(json!({"name": "Alice"}));
        let filter = Filter::EndsWith("name".to_string(), "ice".to_string());
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::EndsWith("name".to_string(), "Bob".to_string());
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_exists() {
        let doc = create_doc(json!({"name": "Alice"}));
        let filter = Filter::Exists("name".to_string(), true);
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::Exists("age".to_string(), true);
        assert!(!matches_filters(&doc, &[filter]));

        let filter = Filter::Exists("age".to_string(), false);
        assert!(matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_and() {
        let doc = create_doc(json!({"name": "Alice", "age": 25}));
        let filter = Filter::And(
            Box::new(Filter::Equals("name".to_string(), json!("Alice"))),
            Box::new(Filter::GreaterThan("age".to_string(), json!(20))),
        );
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::And(
            Box::new(Filter::Equals("name".to_string(), json!("Alice"))),
            Box::new(Filter::GreaterThan("age".to_string(), json!(30))),
        );
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_or() {
        let doc = create_doc(json!({"name": "Alice", "age": 25}));
        let filter = Filter::Or(
            Box::new(Filter::Equals("name".to_string(), json!("Bob"))),
            Box::new(Filter::GreaterThan("age".to_string(), json!(20))),
        );
        assert!(matches_filters(&doc, &[filter]));

        let filter = Filter::Or(
            Box::new(Filter::Equals("name".to_string(), json!("Bob"))),
            Box::new(Filter::GreaterThan("age".to_string(), json!(30))),
        );
        assert!(!matches_filters(&doc, &[filter]));
    }

    #[test]
    fn test_matches_filters_multiple() {
        let doc = create_doc(json!({"name": "Alice", "age": 25}));
        let filters = vec![
            Filter::Equals("name".to_string(), json!("Alice")),
            Filter::GreaterThan("age".to_string(), json!(20)),
        ];
        assert!(matches_filters(&doc, &filters));

        let filters = vec![
            Filter::Equals("name".to_string(), json!("Alice")),
            Filter::GreaterThan("age".to_string(), json!(30)),
        ];
        assert!(!matches_filters(&doc, &filters));
    }
}
