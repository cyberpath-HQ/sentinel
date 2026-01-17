//! Filtering utilities for document matching.

use serde_json::Value;

use crate::{Document, Filter};

/// Checks if a document matches all the given filters.
pub fn matches_filters(doc: &Document, filters: &[&Filter]) -> bool {
    #[allow(
        clippy::needless_borrowed_reference,
        reason = "clippy suggestions are incorrect for matching &Value patterns"
    )]
    for &filter in filters {
        let matches = match *filter {
            Filter::Equals(ref field, ref value) => doc.data().get(field.as_str()) == Some(value),
            Filter::GreaterThan(ref field, ref value) => {
                if let &Value::Number(ref v) = value {
                    if let Some(&Value::Number(ref n)) = doc.data().get(field.as_str()) {
                        n.as_f64().unwrap_or(0.0) > v.as_f64().unwrap_or(0.0)
                    }
                    else {
                        false
                    }
                }
                else {
                    false
                }
            },
            Filter::LessThan(ref field, ref value) => {
                if let &Value::Number(ref v) = value {
                    if let Some(&Value::Number(ref n)) = doc.data().get(field.as_str()) {
                        n.as_f64().unwrap_or(0.0) < v.as_f64().unwrap_or(0.0)
                    }
                    else {
                        false
                    }
                }
                else {
                    false
                }
            },
            Filter::GreaterOrEqual(ref field, ref value) => {
                if let &Value::Number(ref v) = value {
                    if let Some(&Value::Number(ref n)) = doc.data().get(field.as_str()) {
                        n.as_f64().unwrap_or(0.0) >= v.as_f64().unwrap_or(0.0)
                    }
                    else {
                        false
                    }
                }
                else {
                    false
                }
            },
            Filter::LessOrEqual(ref field, ref value) => {
                if let &Value::Number(ref v) = value {
                    if let Some(&Value::Number(ref n)) = doc.data().get(field.as_str()) {
                        n.as_f64().unwrap_or(0.0) <= v.as_f64().unwrap_or(0.0)
                    }
                    else {
                        false
                    }
                }
                else {
                    false
                }
            },
            Filter::In(ref field, ref values) => {
                doc.data()
                    .get(field.as_str())
                    .is_some_and(|v| values.contains(v))
            },
            Filter::Contains(ref field, ref substring) => {
                match doc.data().get(field.as_str()) {
                    Some(&Value::Array(ref arr)) => {
                        arr.iter().any(|v| {
                            if let &Value::String(ref s) = v {
                                s.contains(substring)
                            }
                            else {
                                false
                            }
                        })
                    },
                    Some(&Value::String(ref s)) => s.contains(substring),
                    _ => false,
                }
            },
            Filter::StartsWith(ref field, ref prefix) => {
                match doc.data().get(field.as_str()) {
                    Some(&Value::String(ref s)) => s.starts_with(prefix),
                    _ => false,
                }
            },
            Filter::EndsWith(ref field, ref suffix) => {
                match doc.data().get(field.as_str()) {
                    Some(&Value::String(ref s)) => s.ends_with(suffix),
                    _ => false,
                }
            },
            Filter::Exists(ref field, ref exists) => {
                let field_exists = doc.data().get(field.as_str()).is_some();
                field_exists == *exists
            },
            Filter::And(ref left, ref right) => {
                matches_filters(doc, &[left.as_ref()]) && matches_filters(doc, &[right.as_ref()])
            },
            Filter::Or(ref left, ref right) => {
                matches_filters(doc, &[left.as_ref()]) || matches_filters(doc, &[right.as_ref()])
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

    async fn create_doc(data: Value) -> Document {
        Document::new_without_signature("test".to_string(), data)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_matches_filters_equals() {
        let doc = create_doc(json!({"name": "Alice", "age": 25})).await;
        let filter = Filter::Equals("name".to_string(), json!("Alice"));
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::Equals("name".to_string(), json!("Bob"));
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_greater_than() {
        let doc = create_doc(json!({"age": 25})).await;
        let filter = Filter::GreaterThan("age".to_string(), json!(20));
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::GreaterThan("age".to_string(), json!(30));
        assert!(!matches_filters(&doc, &[&filter]));

        // Test with non-number field
        let doc = create_doc(json!({"name": "Alice"})).await;
        let filter = Filter::GreaterThan("name".to_string(), json!(20));
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_greater_or_equal() {
        let doc = create_doc(json!({"age": 25})).await;
        let filter = Filter::GreaterOrEqual("age".to_string(), json!(25));
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::GreaterOrEqual("age".to_string(), json!(20));
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::GreaterOrEqual("age".to_string(), json!(30));
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_greater_or_equal_non_number() {
        // Test with non-number field
        let doc = create_doc(json!({"name": "Alice"})).await;
        let filter = Filter::GreaterOrEqual("name".to_string(), json!(20));
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_less_than() {
        let doc = create_doc(json!({"age": 25})).await;
        let filter = Filter::LessThan("age".to_string(), json!(20));
        assert!(!matches_filters(&doc, &[&filter]));

        let filter = Filter::LessThan("age".to_string(), json!(30));
        assert!(matches_filters(&doc, &[&filter]));

        // Test with non-number field
        let doc = create_doc(json!({"name": "Alice"})).await;
        let filter = Filter::LessThan("name".to_string(), json!(20));
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_less_or_equal() {
        let doc = create_doc(json!({"age": 25})).await;
        let filter = Filter::LessOrEqual("age".to_string(), json!(25));
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::LessOrEqual("age".to_string(), json!(30));
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::LessOrEqual("age".to_string(), json!(20));
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_less_or_equal_non_number() {
        // Test with non-number field
        let doc = create_doc(json!({"name": "Alice"})).await;
        let filter = Filter::LessOrEqual("name".to_string(), json!(20));
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_in() {
        let doc = create_doc(json!({"status": "active"})).await;
        let filter = Filter::In(
            "status".to_string(),
            vec![json!("active"), json!("inactive")],
        );
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::In("status".to_string(), vec![json!("inactive")]);
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_contains_string() {
        let doc = create_doc(json!({"name": "Alice"})).await;
        let filter = Filter::Contains("name".to_string(), "Ali".to_string());
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::Contains("name".to_string(), "Bob".to_string());
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_contains_array() {
        let doc = create_doc(json!({"tags": ["rust", "programming"]})).await;
        let filter = Filter::Contains("tags".to_string(), "rust".to_string());
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::Contains("tags".to_string(), "python".to_string());
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_contains_array_mixed_types() {
        // Test array with mixed types - should only match strings
        let doc = create_doc(json!({"tags": ["rust", 42, true]})).await;
        let filter = Filter::Contains("tags".to_string(), "rust".to_string());
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::Contains("tags".to_string(), "42".to_string());
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_starts_with() {
        let doc = create_doc(json!({"name": "Alice"})).await;
        let filter = Filter::StartsWith("name".to_string(), "Ali".to_string());
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::StartsWith("name".to_string(), "Bob".to_string());
        assert!(!matches_filters(&doc, &[&filter]));

        // Test with non-string field (should return false)
        let doc = create_doc(json!({"age": 25})).await;
        let filter = Filter::StartsWith("age".to_string(), "2".to_string());
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_ends_with() {
        let doc = create_doc(json!({"name": "Alice"})).await;
        let filter = Filter::EndsWith("name".to_string(), "ice".to_string());
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::EndsWith("name".to_string(), "Bob".to_string());
        assert!(!matches_filters(&doc, &[&filter]));

        // Test with non-string field (should return false)
        let doc = create_doc(json!({"age": 25})).await;
        let filter = Filter::EndsWith("age".to_string(), "5".to_string());
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_exists() {
        let doc = create_doc(json!({"name": "Alice"})).await;
        let filter = Filter::Exists("name".to_string(), true);
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::Exists("age".to_string(), true);
        assert!(!matches_filters(&doc, &[&filter]));

        let filter = Filter::Exists("age".to_string(), false);
        assert!(matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_and() {
        let doc = create_doc(json!({"name": "Alice", "age": 25})).await;
        let filter = Filter::And(
            Box::new(Filter::Equals("name".to_string(), json!("Alice"))),
            Box::new(Filter::GreaterThan("age".to_string(), json!(20))),
        );
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::And(
            Box::new(Filter::Equals("name".to_string(), json!("Alice"))),
            Box::new(Filter::GreaterThan("age".to_string(), json!(30))),
        );
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_or() {
        let doc = create_doc(json!({"name": "Alice", "age": 25})).await;
        let filter = Filter::Or(
            Box::new(Filter::Equals("name".to_string(), json!("Bob"))),
            Box::new(Filter::GreaterThan("age".to_string(), json!(20))),
        );
        assert!(matches_filters(&doc, &[&filter]));

        let filter = Filter::Or(
            Box::new(Filter::Equals("name".to_string(), json!("Bob"))),
            Box::new(Filter::GreaterThan("age".to_string(), json!(30))),
        );
        assert!(!matches_filters(&doc, &[&filter]));
    }

    #[tokio::test]
    async fn test_matches_filters_multiple() {
        let doc = create_doc(json!({"name": "Alice", "age": 25})).await;
        let filters = vec![
            Filter::Equals("name".to_string(), json!("Alice")),
            Filter::GreaterThan("age".to_string(), json!(20)),
        ];
        assert!(matches_filters(&doc, &filters.iter().collect::<Vec<_>>()));

        let filters = vec![
            Filter::Equals("name".to_string(), json!("Alice")),
            Filter::GreaterThan("age".to_string(), json!(30)),
        ];
        assert!(!matches_filters(&doc, &filters.iter().collect::<Vec<_>>()));
    }
}
