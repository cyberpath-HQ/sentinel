//! Filtering utilities for document matching.

use serde_json::Value;

use crate::{Document, Filter};

/// Checks if a document matches all the given filters.
pub fn matches_filters(doc: &Document, filters: &[Filter]) -> bool {
    for filter in filters {
        let matches = match filter {
            Filter::Equals(field, value) => doc.data().get(field).map_or(false, |v| v == value),
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
            Filter::In(field, values) => doc.data().get(field).map_or(false, |v| values.contains(v)),
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
