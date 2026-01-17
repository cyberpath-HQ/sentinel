//! Document projection utilities.

use serde_json::Value;

use crate::Document;

/// Projects a document to include only specified fields.
pub fn project_document(doc: &Document, fields: &[String]) -> Document {
    if fields.is_empty() {
        return doc.clone();
    }
    let mut projected_data = serde_json::Map::new();
    for field in fields {
        if let Some(value) = doc.data().get(field) {
            projected_data.insert(field.clone(), value.clone());
        }
    }
    // Create a new document with projected data
    Document::new_without_signature(doc.id().to_owned(), Value::Object(projected_data)).unwrap_or_else(|_| doc.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_doc(data: Value) -> Document {
        Document::new_without_signature("test".to_string(), data).unwrap()
    }

    #[test]
    fn test_project_document_empty_fields() {
        let doc = create_doc(json!({"name": "Alice", "age": 25}));
        let projected = project_document(&doc, &[]);
        assert_eq!(projected.data(), doc.data());
    }

    #[test]
    fn test_project_document_with_fields() {
        let doc = create_doc(json!({"name": "Alice", "age": 25, "city": "NYC"}));
        let projected = project_document(&doc, &["name".to_string(), "age".to_string()]);
        let expected = json!({"name": "Alice", "age": 25});
        assert_eq!(projected.data(), &expected);
    }

    #[test]
    fn test_project_document_missing_fields() {
        let doc = create_doc(json!({"name": "Alice"}));
        let projected = project_document(&doc, &["name".to_string(), "age".to_string()]);
        let expected = json!({"name": "Alice"});
        assert_eq!(projected.data(), &expected);
    }
}
