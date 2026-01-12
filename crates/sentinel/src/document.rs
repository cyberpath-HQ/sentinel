use serde_json::Value;

/// Represents a document in the database.
pub struct Document {
    /// The unique identifier of the document.
    pub id: String,
    /// The JSON data of the document.
    pub data: Value,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_document_creation() {
        let data = json!({"name": "Test", "value": 42});
        let doc = Document {
            id: "test-id".to_string(),
            data: data.clone(),
        };

        assert_eq!(doc.id, "test-id");
        assert_eq!(doc.data, data);
    }

    #[test]
    fn test_document_with_empty_data() {
        let data = json!({});
        let doc = Document {
            id: "empty".to_string(),
            data,
        };

        assert_eq!(doc.id, "empty");
        assert!(doc.data.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_document_with_complex_data() {
        let data = json!({
            "string": "value",
            "number": 123,
            "boolean": true,
            "array": [1, 2, 3],
            "object": {"nested": "value"}
        });
        let doc = Document {
            id: "complex".to_string(),
            data: data.clone(),
        };

        assert_eq!(doc.data["string"], "value");
        assert_eq!(doc.data["number"], 123);
        assert_eq!(doc.data["boolean"], true);
        assert_eq!(doc.data["array"], json!([1, 2, 3]));
        assert_eq!(doc.data["object"]["nested"], "value");
    }

    #[test]
    fn test_document_id_with_special_characters() {
        let data = json!({"data": "test"});
        let doc = Document {
            id: "user_123-special!".to_string(),
            data,
        };

        assert_eq!(doc.id, "user_123-special!");
    }
}
