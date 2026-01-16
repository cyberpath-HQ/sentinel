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
