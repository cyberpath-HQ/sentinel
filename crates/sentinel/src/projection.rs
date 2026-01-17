//! Document projection utilities.

use serde_json::Value;

use crate::{Document, Result};

/// Projects a document to include only specified fields.
pub async fn project_document(doc: &Document, fields: &[String]) -> Result<Document> {
    if fields.is_empty() {
        return Ok(doc.clone());
    }
    let mut projected_data = serde_json::Map::new();
    for field in fields {
        if let Some(value) = doc.data().get(field) {
            projected_data.insert(field.clone(), value.clone());
        }
    }
    // Create a new document with projected data
    Document::new_without_signature(doc.id().to_owned(), Value::Object(projected_data)).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    async fn create_doc(data: Value) -> Document {
        Document::new_without_signature("test".to_string(), data)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_project_document_empty_fields() {
        let doc = create_doc(json!({"name": "Alice", "age": 25})).await;
        let projected = project_document(&doc, &[]).await.unwrap();
        assert_eq!(projected.data(), doc.data());
    }

    #[tokio::test]
    async fn test_project_document_with_fields() {
        let doc = create_doc(json!({"name": "Alice", "age": 25, "city": "NYC"})).await;
        let projected = project_document(&doc, &["name".to_string(), "age".to_string()])
            .await
            .unwrap();
        let expected = json!({"name": "Alice", "age": 25});
        assert_eq!(projected.data(), &expected);
    }

    #[tokio::test]
    async fn test_project_document_missing_fields() {
        let doc = create_doc(json!({"name": "Alice"})).await;
        let projected = project_document(&doc, &["name".to_string(), "age".to_string()])
            .await
            .unwrap();
        let expected = json!({"name": "Alice"});
        assert_eq!(projected.data(), &expected);
    }
}
