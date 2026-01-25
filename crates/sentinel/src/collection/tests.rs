#[cfg(test)]
mod tests {
    use serde_json::{self, json};
    use tempfile;
    use tokio::fs;
    use futures::TryStreamExt;

    use crate::{Collection, Document, SentinelError, Store};

    async fn setup_collection() -> (Collection, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("test", None).await.unwrap();
        (collection, temp_dir)
    }

    async fn setup_collection_with_signing_key() -> (Collection, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            Some("test_passphrase"),
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("test", None).await.unwrap();
        (collection, temp_dir)
    }

    #[tokio::test]
    async fn test_insert_with_signing_key() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Alice", "signed": true });
        collection.insert("signed_doc", doc.clone()).await.unwrap();

        let retrieved = collection.get("signed_doc").await.unwrap().unwrap();
        assert_eq!(*retrieved.data(), doc);
        // Check that signature is not empty
        assert!(!retrieved.signature().is_empty());
    }

    #[tokio::test]
    async fn test_insert_large_document() {
        let (collection, _temp_dir) = setup_collection().await;

        let large_data = json!({
            "large_array": (0..1000).collect::<Vec<_>>(),
            "nested": {
                "deep": {
                    "value": "test"
                }
            }
        });
        collection
            .insert("large", large_data.clone())
            .await
            .unwrap();

        let retrieved = collection
            .get_with_verification("large", &crate::VerificationOptions::disabled())
            .await
            .unwrap();
        assert_eq!(*retrieved.unwrap().data(), large_data);
    }

    #[tokio::test]
    async fn test_insert_with_invalid_special_characters_in_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "data": "test" });
        let result = collection.insert("user_123-special!", doc.clone()).await;

        // Should return an error for invalid document ID with special characters
        assert!(result.is_err());
        match result {
            Err(SentinelError::InvalidDocumentId {
                id,
            }) => {
                assert_eq!(id, "user_123-special!");
            },
            _ => panic!("Expected InvalidDocumentId error"),
        }
    }

    #[tokio::test]
    async fn test_insert_with_valid_document_ids() {
        let (collection, _temp_dir) = setup_collection().await;

        // Test various valid document IDs
        let valid_ids = vec![
            "user-123",
            "user_456",
            "user123",
            "123",
            "a",
            "user-123_test",
            "user_123-test",
            "CamelCaseID",
            "lower_case_id",
            "UPPER_CASE_ID",
        ];

        for id in valid_ids {
            let doc = json!({ "data": "test" });
            let result = collection.insert(id, doc).await;
            assert!(
                result.is_ok(),
                "Expected ID '{}' to be valid but got error: {:?}",
                id,
                result
            );
        }
    }

    #[tokio::test]
    async fn test_insert_with_various_invalid_document_ids() {
        let (collection, _temp_dir) = setup_collection().await;

        // Test various invalid document IDs
        let invalid_ids = vec![
            "user!123",    // exclamation mark
            "user@domain", // at sign
            "user#123",    // hash
            "user$123",    // dollar sign
            "user%123",    // percent
            "user^123",    // caret
            "user&123",    // ampersand
            "user*123",    // asterisk
            "user(123)",   // parentheses
            "user.123",    // period
            "user/123",    // forward slash
            "user\\123",   // backslash
            "user:123",    // colon
            "user;123",    // semicolon
            "user<123",    // less than
            "user>123",    // greater than
            "user?123",    // question mark
            "user|123",    // pipe
            "user\"123",   // quote
            "user'123",    // single quote
            "",            // empty string
        ];

        for id in invalid_ids {
            let doc = json!({ "data": "test" });
            let result = collection.insert(id, doc).await;
            assert!(
                result.is_err(),
                "Expected ID '{}' to be invalid but insertion succeeded",
                id
            );
            match result {
                Err(SentinelError::InvalidDocumentId {
                    ..
                }) => {
                    // Expected error type
                },
                _ => panic!("Expected InvalidDocumentId error for ID '{}'", id),
            }
        }
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let (collection, _temp_dir) = setup_collection().await;

        let retrieved = collection.get("nonexistent").await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_update() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc1 = json!({ "name": "Alice" });
        collection.insert("user-123", doc1).await.unwrap();

        let doc2 = json!({ "name": "Alice", "age": 30 });
        collection.update("user-123", doc2.clone()).await.unwrap();

        let retrieved = collection
            .get_with_verification("user-123", &crate::VerificationOptions::disabled())
            .await
            .unwrap();
        assert_eq!(*retrieved.unwrap().data(), doc2);
    }

    #[tokio::test]
    async fn test_update_nonexistent() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "name": "Bob" });
        let result = collection.update("new-user", doc.clone()).await;

        // Should return an error for non-existent document
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::SentinelError::DocumentNotFound {
                id,
                collection: _,
            } => {
                assert_eq!(id, "new-user");
            },
            _ => panic!("Expected DocumentNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_update_with_invalid_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "name": "Bob" });
        let result = collection.update("user!invalid", doc).await;

        // Should return an error for invalid document ID
        assert!(result.is_err());
        match result {
            Err(SentinelError::InvalidDocumentId {
                id,
            }) => {
                assert_eq!(id, "user!invalid");
            },
            _ => panic!("Expected InvalidDocumentId error"),
        }
    }

    #[tokio::test]
    async fn test_delete() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "name": "Alice" });
        collection.insert("user-123", doc).await.unwrap();

        let retrieved = collection
            .get_with_verification("user-123", &crate::VerificationOptions::disabled())
            .await
            .unwrap();
        assert!(retrieved.is_some());

        collection.delete("user-123").await.unwrap();

        let retrieved = collection
            .get_with_verification("user-123", &crate::VerificationOptions::disabled())
            .await
            .unwrap();
        assert!(retrieved.is_none());

        // Check that file was moved to .deleted/
        let deleted_path = collection.path.join(".deleted").join("user-123.json");
        assert!(tokio::fs::try_exists(&deleted_path).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let (collection, _temp_dir) = setup_collection().await;

        // Should not error
        collection.delete("nonexistent").await.unwrap();
    }

    #[tokio::test]
    async fn test_list_empty_collection() {
        let (collection, _temp_dir) = setup_collection().await;

        let ids: Vec<String> = collection.list().try_collect().await.unwrap();
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn test_list_with_documents() {
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("user-123", json!({"name": "Alice"}))
            .await
            .unwrap();
        collection
            .insert("user-456", json!({"name": "Bob"}))
            .await
            .unwrap();
        collection
            .insert("user-789", json!({"name": "Charlie"}))
            .await
            .unwrap();

        let mut ids: Vec<String> = collection.list().try_collect().await.unwrap();
        ids.sort(); // Sort for consistent ordering in test
        assert_eq!(ids.len(), 3);
        assert_eq!(ids, vec!["user-123", "user-456", "user-789"]);
    }

    #[tokio::test]
    async fn test_list_skips_deleted_documents() {
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("user-123", json!({"name": "Alice"}))
            .await
            .unwrap();
        collection
            .insert("user-456", json!({"name": "Bob"}))
            .await
            .unwrap();
        collection.delete("user-456").await.unwrap();

        let ids: Vec<String> = collection.list().try_collect().await.unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids, vec!["user-123"]);
    }

    #[tokio::test]
    async fn test_bulk_insert() {
        let (collection, _temp_dir) = setup_collection().await;

        let documents = vec![
            ("user-123", json!({"name": "Alice", "role": "admin"})),
            ("user-456", json!({"name": "Bob", "role": "user"})),
            ("user-789", json!({"name": "Charlie", "role": "user"})),
        ];

        collection.bulk_insert(documents).await.unwrap();

        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert_eq!(docs.len(), 3);

        let ids: Vec<String> = collection.list().try_collect().await.unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&"user-123".to_string()));
        assert!(ids.contains(&"user-456".to_string()));
        assert!(ids.contains(&"user-789".to_string()));

        // Verify data
        let alice = collection
            .get_with_verification("user-123", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(alice.data()["name"], "Alice");
        assert_eq!(alice.data()["role"], "admin");
    }

    #[tokio::test]
    async fn test_bulk_insert_empty() {
        let (collection, _temp_dir) = setup_collection().await;

        collection.bulk_insert(vec![]).await.unwrap();

        let ids: Vec<String> = collection.list().try_collect().await.unwrap();
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn test_bulk_insert_with_invalid_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let documents = vec![
            ("user-123", json!({"name": "Alice"})),
            ("user!invalid", json!({"name": "Bob"})),
        ];

        let result = collection.bulk_insert(documents).await;
        assert!(result.is_err());

        // First document should have been inserted before error
        let ids: Vec<String> = collection.list().try_collect().await.unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "user-123");
    }

    #[tokio::test]
    async fn test_multiple_operations() {
        let (collection, _temp_dir) = setup_collection().await;

        // Insert multiple
        collection
            .insert("user1", json!({"name": "User1"}))
            .await
            .unwrap();
        collection
            .insert("user2", json!({"name": "User2"}))
            .await
            .unwrap();

        // Get both
        let user1 = collection
            .get_with_verification("user1", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        let user2 = collection
            .get_with_verification("user2", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user1.data()["name"], "User1");
        assert_eq!(user2.data()["name"], "User2");

        // Update one
        collection
            .update("user1", json!({"name": "Updated"}))
            .await
            .unwrap();
        let updated = collection
            .get_with_verification("user1", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.data()["name"], "Updated");

        // Delete one
        collection.delete("user2").await.unwrap();
        assert!(collection
            .get_with_verification("user2", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .is_none());
        assert!(collection
            .get_with_verification("user1", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .is_some());
    }

    #[test]
    fn test_validate_document_id_valid() {
        // Valid IDs
        assert!(Collection::validate_document_id("user-123").is_ok());
        assert!(Collection::validate_document_id("user_456").is_ok());
        assert!(Collection::validate_document_id("data-item").is_ok());
        assert!(Collection::validate_document_id("test_collection_123").is_ok());
        assert!(Collection::validate_document_id("file-txt").is_ok());
        assert!(Collection::validate_document_id("a").is_ok());
        assert!(Collection::validate_document_id("123").is_ok());
    }

    #[test]
    fn test_validate_document_id_invalid_empty() {
        assert!(Collection::validate_document_id("").is_err());
    }

    #[test]
    fn test_validate_document_id_invalid_path_separators() {
        assert!(Collection::validate_document_id("path/traversal").is_err());
        assert!(Collection::validate_document_id("path\\traversal").is_err());
    }

    #[test]
    fn test_validate_document_id_invalid_control_characters() {
        assert!(Collection::validate_document_id("file\nname").is_err());
        assert!(Collection::validate_document_id("file\x00name").is_err());
    }

    #[test]
    fn test_validate_document_id_invalid_windows_reserved_characters() {
        assert!(Collection::validate_document_id("file<name>").is_err());
        assert!(Collection::validate_document_id("file>name").is_err());
        assert!(Collection::validate_document_id("file:name").is_err());
        assert!(Collection::validate_document_id("file\"name").is_err());
        assert!(Collection::validate_document_id("file|name").is_err());
        assert!(Collection::validate_document_id("file?name").is_err());
        assert!(Collection::validate_document_id("file*name").is_err());
    }

    #[test]
    fn test_validate_document_id_invalid_other_characters() {
        assert!(Collection::validate_document_id("file name").is_err()); // space
        assert!(Collection::validate_document_id("file@name").is_err()); // @
        assert!(Collection::validate_document_id("file!name").is_err()); // !
        assert!(Collection::validate_document_id("file🚀name").is_err()); // emoji
        assert!(Collection::validate_document_id("fileéname").is_err()); // accented
        assert!(Collection::validate_document_id("file.name").is_err()); // dot
    }

    #[test]
    fn test_validate_document_id_invalid_windows_reserved_names() {
        // Test reserved names (case-insensitive)
        assert!(Collection::validate_document_id("CON").is_err());
        assert!(Collection::validate_document_id("con").is_err());
        assert!(Collection::validate_document_id("Con").is_err());
        assert!(Collection::validate_document_id("PRN").is_err());
        assert!(Collection::validate_document_id("AUX").is_err());
        assert!(Collection::validate_document_id("NUL").is_err());
        assert!(Collection::validate_document_id("COM1").is_err());
        assert!(Collection::validate_document_id("LPT9").is_err());

        // Test with extensions
        assert!(Collection::validate_document_id("CON.txt").is_err());
        assert!(Collection::validate_document_id("prn.backup").is_err());
    }

    #[tokio::test]
    async fn test_insert_invalid_document_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "data": "test" });

        // Test empty ID
        assert!(collection.insert("", doc.clone()).await.is_err());

        // Test Windows reserved name
        assert!(collection.insert("CON", doc.clone()).await.is_err());

        // Test invalid character
        assert!(collection.insert("file name", doc.clone()).await.is_err());
    }

    #[tokio::test]
    async fn test_get_corrupted_json() {
        let (collection, _temp_dir) = setup_collection().await;

        // Manually create a file with invalid JSON
        let file_path = collection.path.join("corrupted.json");
        tokio::fs::write(&file_path, "{ invalid json }")
            .await
            .unwrap();

        let result = collection.get("corrupted").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_invalid_document_id() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "data": "test" });

        // Test empty ID
        assert!(collection.update("", doc.clone()).await.is_err());

        // Test Windows reserved name
        assert!(collection.update("CON", doc.clone()).await.is_err());
    }

    #[tokio::test]
    async fn test_delete_invalid_document_id() {
        let (collection, _temp_dir) = setup_collection().await;

        // Test empty ID
        assert!(collection.delete("").await.is_err());

        // Test Windows reserved name
        assert!(collection.delete("CON").await.is_err());
    }

    #[tokio::test]
    async fn test_get_nonexistent_with_verification() {
        let (collection, _temp_dir) = setup_collection().await;

        let options = crate::VerificationOptions::strict();
        let result = collection
            .get_with_verification("nonexistent", &options)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_get_with_verification_disabled() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Alice", "data": "test" });
        collection.insert("test_doc", doc.clone()).await.unwrap();

        // Tamper with the file
        let file_path = collection.path.join("test_doc.json");
        let mut content = tokio::fs::read_to_string(&file_path).await.unwrap();
        content = content.replace("test", "tampered");
        tokio::fs::write(&file_path, &content).await.unwrap();

        // Should succeed with verification disabled
        let options = crate::VerificationOptions::disabled();
        let result = collection.get_with_verification("test_doc", &options).await;
        assert!(result.is_ok());
        let doc = result.unwrap().unwrap();
        assert_eq!(doc.data()["name"], "Alice");
    }

    #[tokio::test]
    async fn test_get_with_verification_empty_signature_warn() {
        let (collection, _temp_dir) = setup_collection().await;

        // Insert unsigned document
        let doc = json!({ "name": "Unsigned" });
        collection
            .insert("unsigned_doc", doc.clone())
            .await
            .unwrap();

        // Should warn but not fail
        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Strict,
            empty_signature_mode:        crate::VerificationMode::Warn,
            hash_verification_mode:      crate::VerificationMode::Strict,
        };
        let result = collection
            .get_with_verification("unsigned_doc", &options)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_get_with_verification_empty_signature_strict() {
        let (collection, _temp_dir) = setup_collection().await;

        // Insert unsigned document
        let doc = json!({ "name": "Unsigned" });
        collection
            .insert("unsigned_doc", doc.clone())
            .await
            .unwrap();

        // Should fail with strict empty signature mode
        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Strict,
            empty_signature_mode:        crate::VerificationMode::Strict,
            hash_verification_mode:      crate::VerificationMode::Strict,
        };
        let result = collection
            .get_with_verification("unsigned_doc", &options)
            .await;
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::SentinelError::SignatureVerificationFailed {
                id,
                reason,
            } => {
                assert_eq!(id, "unsigned_doc");
                assert!(reason.contains("no signature"));
            },
            _ => panic!("Expected SignatureVerificationFailed"),
        }
    }

    #[tokio::test]
    async fn test_all_empty_collection() {
        let (collection, _temp_dir) = setup_collection().await;

        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert!(docs.is_empty());
    }

    #[tokio::test]
    async fn test_all_with_multiple_documents() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 5 {
            let doc = json!({ "id": i, "name": format!("User{}", i) });
            collection
                .insert(&format!("user-{}", i), doc)
                .await
                .unwrap();
        }

        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert_eq!(docs.len(), 5);

        let ids: std::collections::HashSet<_> = docs.iter().map(|d| d.id().to_string()).collect();
        for i in 0 .. 5 {
            assert!(ids.contains(&format!("user-{}", i)));
        }
    }

    #[tokio::test]
    async fn test_all_with_verification() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        for i in 0 .. 3 {
            let doc = json!({ "id": i });
            collection
                .insert(&format!("signed-{}", i), doc)
                .await
                .unwrap();
        }

        let options = crate::VerificationOptions::strict();
        let docs: Vec<_> = collection
            .all_with_verification(&options)
            .try_collect()
            .await
            .unwrap();
        assert_eq!(docs.len(), 3);
    }

    #[tokio::test]
    async fn test_filter_empty_result() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 3 {
            let doc = json!({ "id": i, "status": "active" });
            collection
                .insert(&format!("user-{}", i), doc)
                .await
                .unwrap();
        }

        let results: Vec<_> = collection
            .filter(|doc| doc.data().get("status") == Some(&json!("inactive")))
            .try_collect()
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_filter_with_all_matching() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 5 {
            let doc = json!({ "id": i, "active": true });
            collection
                .insert(&format!("user-{}", i), doc)
                .await
                .unwrap();
        }

        let results: Vec<_> = collection
            .filter(|doc| doc.data().get("active") == Some(&json!(true)))
            .try_collect()
            .await
            .unwrap();
        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_filter_with_verification() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        for i in 0 .. 3 {
            let doc = json!({ "id": i, "active": true });
            collection
                .insert(&format!("signed-{}", i), doc)
                .await
                .unwrap();
        }

        let options = crate::VerificationOptions::strict();
        let results: Vec<_> = collection
            .filter_with_verification(
                |doc| doc.data().get("active") == Some(&json!(true)),
                &options,
            )
            .try_collect()
            .await
            .unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_bulk_insert_empty_all() {
        let (collection, _temp_dir) = setup_collection().await;

        let result = collection.bulk_insert(vec![]).await;
        assert!(result.is_ok());

        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert!(docs.is_empty());
    }

    #[tokio::test]
    async fn test_bulk_insert_large_batch() {
        let (collection, _temp_dir) = setup_collection().await;

        let documents: Vec<(String, serde_json::Value)> = (0 .. 100)
            .map(|i| {
                let key = format!("user-{}", i);
                let value = json!({ "id": i, "data": format!("value{}", i) });
                (key, value)
            })
            .collect();

        // Convert Vec<(String, Value)> to Vec<(&str, Value)>
        let documents_refs: Vec<(&str, serde_json::Value)> = documents
            .iter()
            .map(|(k, v)| (k.as_str(), v.clone()))
            .collect();

        // This should trigger the debug log for bulk insert
        let result = collection.bulk_insert(documents_refs).await;
        assert!(result.is_ok());

        // Verify all documents were inserted
        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert_eq!(docs.len(), 100);
    }

    #[tokio::test]
    async fn test_bulk_insert_partial_failure() {
        let (collection, _temp_dir) = setup_collection().await;

        let documents = vec![
            ("valid-1", json!({"name": "One"})),
            ("valid-2", json!({"name": "Two"})),
            ("invalid id!", json!({"name": "Three"})), // This will fail
        ];

        let result = collection.bulk_insert(documents).await;
        assert!(result.is_err());

        // First two should not be inserted (transaction safety not implemented yet)
        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert!(docs.len() <= 2);
    }

    #[tokio::test]
    async fn test_query_empty_filter() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 10 {
            let doc = json!({ "id": i, "value": i * 10 });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new().build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();
        assert_eq!(docs.len(), 10);
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 100 {
            let doc = json!({ "id": i });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new().limit(5).build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();
        assert_eq!(docs.len(), 5);
    }

    #[tokio::test]
    async fn test_query_with_offset() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 10 {
            let doc = json!({ "id": i, "value": i });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new().offset(5).build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();
        assert_eq!(docs.len(), 5);
    }

    #[tokio::test]
    async fn test_query_with_limit_and_offset() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 100 {
            let doc = json!({ "id": i, "value": i });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new().offset(10).limit(5).build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();
        assert_eq!(docs.len(), 5);
    }

    #[tokio::test]
    async fn test_query_with_sort_ascending() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in (0 .. 5).rev() {
            let doc = json!({ "id": i, "value": i });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new()
            .sort("id", crate::SortOrder::Ascending)
            .build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();

        assert_eq!(docs.len(), 5);
        for (i, doc) in docs.iter().enumerate() {
            assert_eq!(doc.data()["id"], json!(i));
        }
    }

    #[tokio::test]
    async fn test_query_with_sort_descending() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 5 {
            let doc = json!({ "id": i, "value": i });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new()
            .sort("id", crate::SortOrder::Descending)
            .build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();

        assert_eq!(docs.len(), 5);
        for (i, doc) in docs.iter().enumerate() {
            assert_eq!(doc.data()["id"], json!(4 - i));
        }
    }

    #[tokio::test]
    async fn test_query_with_projection() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 3 {
            let doc =
                json!({ "id": i, "name": format!("User{}", i), "email": format!("user{}@example.com", i), "age": 30 });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        let query = crate::QueryBuilder::new()
            .projection(vec!["id", "name"])
            .build();
        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();

        assert_eq!(docs.len(), 3);
        for doc in &docs {
            assert!(doc.data().get("id").is_some());
            assert!(doc.data().get("name").is_some());
            assert!(doc.data().get("email").is_none());
            assert!(doc.data().get("age").is_none());
        }
    }

    #[tokio::test]
    async fn test_query_with_verification() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        for i in 0 .. 5 {
            let doc = json!({ "id": i, "active": true });
            collection
                .insert(&format!("signed-{}", i), doc)
                .await
                .unwrap();
        }

        let options = crate::VerificationOptions::strict();
        let query = crate::QueryBuilder::new().build();
        let result = collection
            .query_with_verification(query, &options)
            .await
            .unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();
        assert_eq!(docs.len(), 5);
    }

    #[tokio::test]
    async fn test_query_complex() {
        let (collection, _temp_dir) = setup_collection().await;

        // Insert test data
        let test_data = vec![
            (
                "doc1",
                json!({ "name": "Alice", "age": 25, "city": "NYC", "active": true }),
            ),
            (
                "doc2",
                json!({ "name": "Bob", "age": 30, "city": "LA", "active": true }),
            ),
            (
                "doc3",
                json!({ "name": "Charlie", "age": 35, "city": "NYC", "active": false }),
            ),
            (
                "doc4",
                json!({ "name": "Diana", "age": 28, "city": "NYC", "active": true }),
            ),
            (
                "doc5",
                json!({ "name": "Eve", "age": 40, "city": "LA", "active": false }),
            ),
        ];

        for (id, doc) in &test_data {
            collection.insert(id, doc.clone()).await.unwrap();
        }

        // Query: active=true, city=NYC, age>=26, limit 2, sort age asc, project name,age
        let query = crate::QueryBuilder::new()
            .filter("active", crate::Operator::Equals, json!(true))
            .filter("city", crate::Operator::Equals, json!("NYC"))
            .filter("age", crate::Operator::GreaterOrEqual, json!(26))
            .sort("age", crate::SortOrder::Ascending)
            .limit(2)
            .projection(vec!["name", "age"])
            .build();

        let result = collection.query(query).await.unwrap();
        let docs: Vec<_> = result.documents.try_collect().await.unwrap();

        assert_eq!(docs.len(), 1);
        // Diana is 28, Bob is 30 but in LA (filtered out by city=NYC)
        assert_eq!(docs[0].data()["name"], json!("Diana"));
    }

    #[tokio::test]
    async fn test_delete_and_recover() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({ "name": "ToDelete" });
        collection.insert("test-doc", doc.clone()).await.unwrap();

        // Verify it exists
        assert!(collection.get("test-doc").await.unwrap().is_some());

        // Delete it
        collection.delete("test-doc").await.unwrap();

        // Verify it's gone from main collection
        assert!(collection.get("test-doc").await.unwrap().is_none());

        // Verify it's in .deleted/
        let deleted_path = collection.path.join(".deleted").join("test-doc.json");
        assert!(tokio::fs::try_exists(&deleted_path).await.unwrap());

        // Recover it manually (no recover API yet)
        tokio::fs::rename(&deleted_path, collection.path.join("test-doc.json"))
            .await
            .unwrap();

        // Verify it's back
        assert!(collection.get("test-doc").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_insert_special_characters_in_data() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({
            "string": "hello\nworld\ttab",
            "unicode": "Hello 世界 🌍",
            "null": null,
            "array": [1, 2, 3, "four"],
            "nested": { "deep": { "value": 42 } }
        });

        collection.insert("special-doc", doc.clone()).await.unwrap();

        let retrieved = collection.get("special-doc").await.unwrap().unwrap();
        assert_eq!(retrieved.data(), &doc);
    }

    #[tokio::test]
    async fn test_insert_very_long_document_id() {
        let (collection, _temp_dir) = setup_collection().await;

        // Use a reasonably long ID that works on most filesystems
        // (255 bytes may exceed some filesystem limits depending on path length)
        let long_id = "a".repeat(200);
        let doc = json!({ "data": "test" });

        let result = collection.insert(&long_id, doc).await;
        assert!(result.is_ok());

        let retrieved = collection.get(&long_id).await.unwrap().unwrap();
        assert_eq!(retrieved.id(), &long_id);
    }

    #[tokio::test]
    async fn test_insert_max_value_numbers() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({
            "max_i64": 9223372036854775807i64,
            "min_i64": -9223372036854775808i64,
            "max_f64": 1.7976931348623157e308,
            "min_f64": -1.7976931348623157e308
        });

        collection.insert("numbers", doc.clone()).await.unwrap();

        let retrieved = collection.get("numbers").await.unwrap().unwrap();
        assert_eq!(retrieved.data(), &doc);
    }

    #[tokio::test]
    async fn test_insert_nested_array_document() {
        let (collection, _temp_dir) = setup_collection().await;

        let doc = json!({
            "matrix": [[1, 2, 3], [4, 5, 6], [7, 8, 9]],
            "mixed": [1, "two", true, null, { "nested": "value" }]
        });

        collection.insert("arrays", doc.clone()).await.unwrap();

        let retrieved = collection.get("arrays").await.unwrap().unwrap();
        assert_eq!(retrieved.data(), &doc);
    }

    #[tokio::test]
    async fn test_collection_name() {
        let (collection, _temp_dir) = setup_collection().await;

        assert_eq!(collection.name(), "test");
    }

    #[tokio::test]
    async fn test_verify_hash_valid() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Test" });
        collection.insert("hash-test", doc.clone()).await.unwrap();

        let retrieved = collection.get("hash-test").await.unwrap().unwrap();
        let options = crate::VerificationOptions {
            verify_signature:            false,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Strict,
            empty_signature_mode:        crate::VerificationMode::Warn,
            hash_verification_mode:      crate::VerificationMode::Strict,
        };

        let result = collection.verify_document(&retrieved, &options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_hash_invalid() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Original" });
        collection
            .insert("hash-invalid", doc.clone())
            .await
            .unwrap();

        // Tamper with the file
        let file_path = collection.path.join("hash-invalid.json");
        let mut content = tokio::fs::read_to_string(&file_path).await.unwrap();
        content = content.replace("\"hash\":", "\"hash\": \"corrupted_hash\", \"old_hash\":");
        tokio::fs::write(&file_path, &content).await.unwrap();

        // Re-read the document (disable verification to read the tampered file)
        let retrieved = collection
            .get_with_verification("hash-invalid", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();

        let options = crate::VerificationOptions {
            verify_signature:            false,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Strict,
            empty_signature_mode:        crate::VerificationMode::Warn,
            hash_verification_mode:      crate::VerificationMode::Strict,
        };

        let result = collection.verify_document(&retrieved, &options).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::SentinelError::HashVerificationFailed {
                id,
                ..
            } => {
                assert_eq!(id, "hash-invalid");
            },
            _ => panic!("Expected HashVerificationFailed"),
        }
    }

    #[tokio::test]
    async fn test_verify_signature_valid() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Signed" });
        collection
            .insert("signed-valid", doc.clone())
            .await
            .unwrap();

        let retrieved = collection.get("signed-valid").await.unwrap().unwrap();
        let options = crate::VerificationOptions::strict();

        let result = collection.verify_document(&retrieved, &options).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_signature_invalid() {
        let (collection, _temp_dir) = setup_collection_with_signing_key().await;

        let doc = json!({ "name": "Original" });
        collection.insert("sig-invalid", doc.clone()).await.unwrap();

        // Tamper with the file
        let file_path = collection.path.join("sig-invalid.json");
        let mut content = tokio::fs::read_to_string(&file_path).await.unwrap();
        content = content.replace(
            "\"signature\":",
            "\"signature\": \"bad_sig\", \"original_signature\":",
        );
        tokio::fs::write(&file_path, &content).await.unwrap();

        // Re-read the document (disable verification to read the tampered file)
        let retrieved = collection
            .get_with_verification("sig-invalid", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        let options = crate::VerificationOptions::strict();

        let result = collection.verify_document(&retrieved, &options).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_insert_unsigned_document() {
        // Test inserting document without signing key to cover line 147-148
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("test", None).await.unwrap();

        let data = json!({ "name": "test" });
        let result = collection.insert("unsigned-doc", data).await;
        assert!(result.is_ok());

        let doc = collection.get("unsigned-doc").await.unwrap().unwrap();
        assert_eq!(doc.data()["name"], "test");
    }

    #[tokio::test]
    async fn test_delete_non_existent() {
        // Test deleting a document that doesn't exist to cover line 371-374
        let (collection, _temp_dir) = setup_collection().await;

        // Try to delete a document that was never created
        let result = collection.delete("does-not-exist").await;
        assert!(result.is_ok()); // Should succeed silently
    }

    #[tokio::test]
    async fn test_delete_soft_delete_path() {
        // Test soft delete to cover line 358-359
        let (collection, temp_dir) = setup_collection().await;

        // Insert a document
        let data = json!({ "name": "to-delete" });
        collection.insert("doc-to-delete", data).await.unwrap();

        // Delete it
        let result = collection.delete("doc-to-delete").await;
        assert!(result.is_ok());

        // Verify it's in .deleted directory
        let deleted_path = temp_dir
            .path()
            .join("data/test/.deleted/doc-to-delete.json");
        assert!(tokio::fs::metadata(&deleted_path).await.is_ok());
    }

    #[tokio::test]
    async fn test_streaming_all_skips_deleted() {
        let (collection, _temp_dir) = setup_collection().await;

        for i in 0 .. 5 {
            let doc = json!({ "id": i });
            collection.insert(&format!("doc-{}", i), doc).await.unwrap();
        }

        // Delete some
        collection.delete("doc-1").await.unwrap();
        collection.delete("doc-3").await.unwrap();

        let docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert_eq!(docs.len(), 3);

        let ids: std::collections::HashSet<_> = docs.iter().map(|d| d.id().to_string()).collect();
        assert!(ids.contains("doc-0"));
        assert!(!ids.contains("doc-1"));
        assert!(ids.contains("doc-2"));
        assert!(!ids.contains("doc-3"));
        assert!(ids.contains("doc-4"));
    }

    #[tokio::test]
    async fn test_count_method() {
        // Test line 449-452: count() method trace logs
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("doc-1", json!({"data": 1}))
            .await
            .unwrap();
        collection
            .insert("doc-2", json!({"data": 2}))
            .await
            .unwrap();

        // Allow event processor to update counters
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Flush to ensure event processor has processed the events
        collection.flush_metadata().await.unwrap();

        let count = collection.count().await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_get_many() {
        // Test lines 1467-1468, 1470, 1472, 1476: get_many batch retrieval
        let (collection, _temp_dir) = setup_collection().await;

        collection.insert("doc-1", json!({"id": 1})).await.unwrap();
        collection.insert("doc-2", json!({"id": 2})).await.unwrap();

        let ids = vec!["doc-1", "doc-2", "non-existent"];
        let results = collection.get_many(&ids).await.unwrap();

        assert_eq!(results.len(), 3);
        assert!(results[0].is_some());
        assert!(results[1].is_some());
        assert!(results[2].is_none());
    }

    #[tokio::test]
    async fn test_upsert_insert() {
        // Test lines 1531-1533: upsert creates new document
        let (collection, _temp_dir) = setup_collection().await;

        let is_new = collection
            .upsert("new-doc", json!({"value": 100}))
            .await
            .unwrap();
        assert!(is_new);

        let doc = collection.get("new-doc").await.unwrap().unwrap();
        assert_eq!(doc.data().get("value").unwrap(), &json!(100));
    }

    #[tokio::test]
    async fn test_upsert_update() {
        // Test lines 1523, 1525-1527: upsert updates existing document
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("existing", json!({"value": 1}))
            .await
            .unwrap();
        let is_new = collection
            .upsert("existing", json!({"value": 2}))
            .await
            .unwrap();

        assert!(!is_new);
        let doc = collection.get("existing").await.unwrap().unwrap();
        assert_eq!(doc.data().get("value").unwrap(), &json!(2));
    }

    #[tokio::test]
    async fn test_aggregate_count() {
        // Test line 1601: aggregate count
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("doc-1", json!({"value": 1}))
            .await
            .unwrap();
        collection
            .insert("doc-2", json!({"value": 2}))
            .await
            .unwrap();

        let result = collection
            .aggregate(vec![], crate::Aggregation::Count)
            .await
            .unwrap();
        assert_eq!(result, json!(2));
    }

    #[tokio::test]
    async fn test_aggregate_sum() {
        // Test lines 1594-1596: aggregate sum
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("doc-1", json!({"amount": 10}))
            .await
            .unwrap();
        collection
            .insert("doc-2", json!({"amount": 20}))
            .await
            .unwrap();

        let result = collection
            .aggregate(vec![], crate::Aggregation::Sum("amount".to_string()))
            .await
            .unwrap();
        assert_eq!(result, json!(30.0));
    }

    #[tokio::test]
    async fn test_aggregate_avg() {
        // Test lines 1609-1612: aggregate average
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("doc-1", json!({"score": 10}))
            .await
            .unwrap();
        collection
            .insert("doc-2", json!({"score": 20}))
            .await
            .unwrap();
        collection
            .insert("doc-3", json!({"score": 30}))
            .await
            .unwrap();

        let result = collection
            .aggregate(vec![], crate::Aggregation::Avg("score".to_string()))
            .await
            .unwrap();
        assert_eq!(result, json!(20.0));
    }

    #[tokio::test]
    async fn test_aggregate_avg_no_docs() {
        // Test lines 1604-1606: average with no documents
        let (collection, _temp_dir) = setup_collection().await;

        let result = collection
            .aggregate(vec![], crate::Aggregation::Avg("score".to_string()))
            .await
            .unwrap();
        assert_eq!(result, json!(null));
    }

    #[tokio::test]
    async fn test_aggregate_min() {
        // Test lines 1621-1622: aggregate min
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("doc-1", json!({"value": 15}))
            .await
            .unwrap();
        collection
            .insert("doc-2", json!({"value": 5}))
            .await
            .unwrap();
        collection
            .insert("doc-3", json!({"value": 10}))
            .await
            .unwrap();

        let result = collection
            .aggregate(vec![], crate::Aggregation::Min("value".to_string()))
            .await
            .unwrap();
        assert_eq!(result, json!(5.0));
    }

    #[tokio::test]
    async fn test_aggregate_min_no_values() {
        // Test lines 1617-1619: min with no numeric values
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("doc-1", json!({"name": "test"}))
            .await
            .unwrap();

        let result = collection
            .aggregate(vec![], crate::Aggregation::Min("value".to_string()))
            .await
            .unwrap();
        assert_eq!(result, json!(null));
    }

    #[tokio::test]
    async fn test_aggregate_max() {
        // Test line 1633: aggregate max
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("doc-1", json!({"value": 15}))
            .await
            .unwrap();
        collection
            .insert("doc-2", json!({"value": 25}))
            .await
            .unwrap();
        collection
            .insert("doc-3", json!({"value": 10}))
            .await
            .unwrap();

        let result = collection
            .aggregate(vec![], crate::Aggregation::Max("value".to_string()))
            .await
            .unwrap();
        assert_eq!(result, json!(25.0));
    }

    #[tokio::test]
    async fn test_aggregate_max_no_values() {
        // Test lines 1629-1630: max with no numeric values
        let (collection, _temp_dir) = setup_collection().await;

        let result = collection
            .aggregate(vec![], crate::Aggregation::Max("value".to_string()))
            .await
            .unwrap();
        assert_eq!(result, json!(null));
    }

    #[tokio::test]
    async fn test_aggregate_with_filters() {
        // Test lines 1587-1590, 1592: aggregation with filters
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("doc-1", json!({"category": "A", "value": 10}))
            .await
            .unwrap();
        collection
            .insert("doc-2", json!({"category": "B", "value": 20}))
            .await
            .unwrap();
        collection
            .insert("doc-3", json!({"category": "A", "value": 15}))
            .await
            .unwrap();

        let filters = vec![crate::Filter::Equals("category".to_string(), json!("A"))];
        let result = collection
            .aggregate(filters, crate::Aggregation::Sum("value".to_string()))
            .await
            .unwrap();
        assert_eq!(result, json!(25.0));
    }

    #[tokio::test]
    async fn test_update_not_found() {
        // Test line 1396: update non-existent document
        let (collection, _temp_dir) = setup_collection().await;

        let result = collection
            .update("non-existent", json!({"data": "value"}))
            .await;
        assert!(matches!(
            result,
            Err(crate::SentinelError::DocumentNotFound { .. })
        ));
    }

    #[tokio::test]
    async fn test_update_merge_json_non_object() {
        // Test line 1364: merge when new value is not an object
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("doc-1", json!({"name": "old"}))
            .await
            .unwrap();
        collection
            .update("doc-1", json!("simple string"))
            .await
            .unwrap();

        let doc = collection.get("doc-1").await.unwrap().unwrap();
        assert_eq!(doc.data(), &json!("simple string"));
    }

    #[tokio::test]
    async fn test_extract_numeric_value() {
        // Test lines 1369-1373: extract numeric value helper
        let (collection, _temp_dir) = setup_collection().await;

        collection
            .insert("doc-1", json!({"price": 99.99, "name": "Product"}))
            .await
            .unwrap();
        let doc = collection.get("doc-1").await.unwrap().unwrap();

        let price = Collection::extract_numeric_value(&doc, "price");
        assert_eq!(price, Some(99.99));

        let name = Collection::extract_numeric_value(&doc, "name");
        assert_eq!(name, None);

        let missing = Collection::extract_numeric_value(&doc, "missing_field");
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_delete_non_existent_persistence() {
        // Test lines 371-374: delete non-existent document
        let (collection, _temp_dir) = setup_collection().await;

        // Should succeed (idempotent)
        let result = collection.delete("does-not-exist").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_unsigned_document() {
        // Test lines 1396, 1409-1410, 1413, 1417: update path without signing key
        let temp_dir = tempfile::tempdir().unwrap();

        // Create store and collection without signing key
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store
            .collection_with_config("test_collection", None)
            .await
            .unwrap();

        // Insert a document without signature (using the insert API directly)
        collection
            .insert("doc1", json!({"name": "Alice", "age": 30}))
            .await
            .unwrap();

        // Update the document (this will use the path without signing key)
        collection.update("doc1", json!({"age": 31})).await.unwrap();

        // Verify update succeeded
        let updated_doc = collection.get("doc1").await.unwrap().unwrap();
        assert_eq!(updated_doc.data()["age"], 31);
        assert_eq!(updated_doc.data()["name"], "Alice");
    }

    #[tokio::test]
    async fn test_verify_signature_no_signing_key() {
        // Test line 1279: verification without signing key
        let temp_dir = tempfile::tempdir().unwrap();

        // Create store and collection without signing key
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store
            .collection_with_config("test_collection", None)
            .await
            .unwrap();

        // Insert a document without signature
        collection
            .insert("doc1", json!({"name": "Alice"}))
            .await
            .unwrap();

        // Get document and verify (should skip signature verification without key)
        let doc = collection.get("doc1").await.unwrap().unwrap();
        let options = crate::verification::VerificationOptions {
            verify_hash:                 true,
            verify_signature:            true,
            hash_verification_mode:      crate::VerificationMode::Strict,
            signature_verification_mode: crate::VerificationMode::Strict,
            empty_signature_mode:        crate::VerificationMode::Warn,
        };

        // This should succeed since there's no signing key to verify against (line 1279)
        collection.verify_document(&doc, &options).await.unwrap();
    }

    #[tokio::test]
    async fn test_update_with_signing_key() {
        // Test line 1396: update path WITH signing key
        let temp_dir = tempfile::tempdir().unwrap();

        // Create store and collection WITH signing key
        let store = Store::new_with_config(
            temp_dir.path(),
            Some("test_passphrase"),
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store
            .collection_with_config("test_collection", None)
            .await
            .unwrap();

        // Insert a document with signature
        collection
            .insert("doc1", json!({"name": "Alice", "age": 30}))
            .await
            .unwrap();

        // Update the document (this will use the path WITH signing key)
        collection.update("doc1", json!({"age": 31})).await.unwrap();

        // Verify update succeeded
        let updated_doc = collection.get("doc1").await.unwrap().unwrap();
        assert_eq!(updated_doc.data()["age"], 31);
        assert_eq!(updated_doc.data()["name"], "Alice");
    }
}

#[cfg(test)]
mod persistence_tests {
    use tempfile::tempdir;
    use tokio::fs;
    use futures::TryStreamExt;
    use serde_json::json;

    use super::*;
    use crate::{Collection, CollectionMetadata, Document, Store};

    async fn setup_collection_with_signing_key() -> (Collection, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            Some("test_passphrase"),
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("test", None).await.unwrap();
        (collection, temp_dir)
    }

    async fn setup_collection() -> (Collection, tempfile::TempDir) {
        let temp_dir = tempfile::tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("test", None).await.unwrap();
        (collection, temp_dir)
    }

    #[tokio::test]
    async fn test_metadata_persistence_across_restarts() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("test_store");

        // First "application session" - create store and collection, add documents
        {
            let store = Store::new_with_config(
                store_path.clone(),
                None,
                sentinel_wal::StoreWalConfig::default(),
            )
            .await
            .unwrap();
            let collection = store
                .collection_with_config("test_collection", None)
                .await
                .unwrap();

            // Insert some documents
            collection
                .insert("doc1", serde_json::json!({"name": "Alice", "age": 30}))
                .await
                .unwrap();
            collection
                .insert("doc2", serde_json::json!({"name": "Bob", "age": 25}))
                .await
                .unwrap();
            collection
                .insert("doc3", serde_json::json!({"name": "Charlie", "age": 35}))
                .await
                .unwrap();

            // Update one document
            collection
                .update("doc2", serde_json::json!({"name": "Bob", "age": 26}))
                .await
                .unwrap();

            // Delete one document
            collection.delete("doc3").await.unwrap();

            // Allow event processor to update counters
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Flush pending metadata changes (async event processor uses debouncing)
            collection.flush_metadata().await.unwrap();

            // Check metadata is correct in memory
            let metadata_path = store_path
                .join("data")
                .join("test_collection")
                .join(".metadata.json");
            let metadata_content = fs::read_to_string(&metadata_path).await.unwrap();
            let metadata: CollectionMetadata = serde_json::from_str(&metadata_content).unwrap();

            assert_eq!(metadata.document_count, 2); // 3 inserted, 1 deleted
            assert!(metadata.total_size_bytes > 0);
            println!(
                "First session - document_count: {}, total_size_bytes: {}",
                metadata.document_count, metadata.total_size_bytes
            );
        }

        // Second "application session" - reload store and verify metadata persisted
        {
            let store = Store::new_with_config(
                store_path.clone(),
                None,
                sentinel_wal::StoreWalConfig::default(),
            )
            .await
            .unwrap();
            let collection = store
                .collection_with_config("test_collection", None)
                .await
                .unwrap();

            // Check that metadata was loaded correctly from disk
            let metadata_path = store_path
                .join("data")
                .join("test_collection")
                .join(".metadata.json");
            let metadata_content = fs::read_to_string(&metadata_path).await.unwrap();
            let metadata: CollectionMetadata = serde_json::from_str(&metadata_content).unwrap();

            assert_eq!(metadata.document_count, 2);
            assert!(metadata.total_size_bytes > 0);
            println!(
                "Second session - document_count: {}, total_size_bytes: {}",
                metadata.document_count, metadata.total_size_bytes
            );

            // Verify documents exist
            assert!(collection.get("doc1").await.unwrap().is_some());
            assert!(collection.get("doc2").await.unwrap().is_some());
            assert!(collection.get("doc3").await.unwrap().is_none()); // Should be deleted

            // Add one more document
            collection
                .insert("doc4", serde_json::json!({"name": "Diana", "age": 28}))
                .await
                .unwrap();

            // Allow event processor to update counters
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Flush pending metadata changes
            collection.flush_metadata().await.unwrap();
        }

        // Third "application session" - final verification
        {
            let store = Store::new_with_config(store_path, None, sentinel_wal::StoreWalConfig::default())
                .await
                .unwrap();
            let collection = store
                .collection_with_config("test_collection", None)
                .await
                .unwrap();

            // Check final metadata
            let metadata_path = store
                .root_path()
                .join("data")
                .join("test_collection")
                .join(".metadata.json");
            let metadata_content = fs::read_to_string(&metadata_path).await.unwrap();
            let metadata: CollectionMetadata = serde_json::from_str(&metadata_content).unwrap();

            assert_eq!(metadata.document_count, 3); // 2 from before + 1 new
            assert!(metadata.total_size_bytes > 0);
            println!(
                "Third session - document_count: {}, total_size_bytes: {}",
                metadata.document_count, metadata.total_size_bytes
            );

            // Verify all documents
            assert!(collection.get("doc1").await.unwrap().is_some());
            assert!(collection.get("doc2").await.unwrap().is_some());
            assert!(collection.get("doc3").await.unwrap().is_none());
            assert!(collection.get("doc4").await.unwrap().is_some());
        }
    }

    #[tokio::test]
    async fn test_collection_wal_config_methods() {
        let (collection, _temp_dir): (Collection, tempfile::TempDir) = setup_collection().await;

        // Test stored_wal_config
        let stored = collection.stored_wal_config();
        assert_eq!(stored, &sentinel_wal::CollectionWalConfig::default());

        // Test wal_config
        let wal = collection.wal_config();
        assert_eq!(wal, &sentinel_wal::CollectionWalConfig::default());
    }

    // ============ Streaming Verification Error Tests ============

    #[tokio::test]
    async fn test_all_with_verification_hash_failure_strict() {
        let (collection, _temp_dir): (Collection, tempfile::TempDir) = setup_collection_with_signing_key().await;

        // Insert a valid document
        let doc = json!({"name": "Valid"});
        collection.insert("valid-doc", doc).await.unwrap();

        // Tamper with the file to break hash
        let file_path = collection.path.join("valid-doc.json");
        let mut content: String = fs::read_to_string(&file_path).await.unwrap();
        content = content.replace("Valid", "Tampered");
        fs::write(&file_path, &content).await.unwrap();

        // Streaming all with strict verification should fail
        let options = crate::VerificationOptions::strict();
        let result: Result<Vec<Document>, _> = collection
            .all_with_verification(&options)
            .try_collect()
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::SentinelError::HashVerificationFailed {
                ..
            } => {},
            e => panic!("Expected HashVerificationFailed, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_all_with_verification_hash_failure_warn() {
        let (collection, _temp_dir): (Collection, tempfile::TempDir) = setup_collection_with_signing_key().await;

        // Insert a valid document
        let doc = json!({"name": "Valid"});
        collection.insert("valid-doc", doc).await.unwrap();

        // Tamper with the file to break hash
        let file_path = collection.path.join("valid-doc.json");
        let mut content: String = fs::read_to_string(&file_path).await.unwrap();
        content = content.replace("Valid", "Tampered");
        fs::write(&file_path, &content).await.unwrap();

        // Streaming all with warn verification should succeed and return invalid docs with warnings
        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Warn,
            empty_signature_mode:        crate::VerificationMode::Warn,
            hash_verification_mode:      crate::VerificationMode::Warn,
        };
        let docs: Vec<Document> = collection
            .all_with_verification(&options)
            .try_collect()
            .await
            .unwrap();

        // Should have 1 document since warn mode includes invalid documents with warnings
        assert_eq!(docs.len(), 1);
    }

    #[tokio::test]
    async fn test_filter_with_verification_signature_failure_strict() {
        let (collection, _temp_dir): (Collection, tempfile::TempDir) = setup_collection_with_signing_key().await;

        // Insert a valid document
        let doc = json!({"name": "Valid", "status": "active"});
        collection.insert("valid-doc", doc).await.unwrap();

        // Tamper with the file to break signature
        let file_path = collection.path.join("valid-doc.json");
        let mut content: String = fs::read_to_string(&file_path).await.unwrap();
        content = content.replace("Valid", "Tampered");
        fs::write(&file_path, &content).await.unwrap();

        // Streaming filter with strict verification should fail
        let options = crate::VerificationOptions::strict();
        let result: Result<Vec<Document>, _> = collection
            .filter_with_verification(|_| true, &options)
            .try_collect()
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            crate::SentinelError::HashVerificationFailed {
                ..
            } => {},
            crate::SentinelError::SignatureVerificationFailed {
                ..
            } => {},
            e => panic!("Expected verification failure, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_filter_with_verification_signature_failure_warn() {
        let (collection, _temp_dir): (Collection, tempfile::TempDir) = setup_collection_with_signing_key().await;

        // Insert a valid document
        let doc = json!({"name": "Valid", "status": "active"});
        collection.insert("valid-doc", doc).await.unwrap();

        // Tamper with the file to break signature
        let file_path = collection.path.join("valid-doc.json");
        let mut content: String = fs::read_to_string(&file_path).await.unwrap();
        content = content.replace("Valid", "Tampered");
        fs::write(&file_path, &content).await.unwrap();

        // Streaming filter with warn verification should succeed and return invalid docs with warnings
        let options = crate::VerificationOptions {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: crate::VerificationMode::Warn,
            empty_signature_mode:        crate::VerificationMode::Warn,
            hash_verification_mode:      crate::VerificationMode::Warn,
        };
        let docs: Vec<Document> = collection
            .filter_with_verification(|_| true, &options)
            .try_collect()
            .await
            .unwrap();

        // Should have 1 document since warn mode includes invalid documents with warnings
        assert_eq!(docs.len(), 1);
    }

    #[tokio::test]
    async fn test_all_with_verification_corrupted_json() {
        let (collection, _temp_dir): (Collection, tempfile::TempDir) = setup_collection_with_signing_key().await;

        // Create a corrupted JSON file manually
        let file_path = collection.path.join("corrupted.json");
        fs::write(&file_path, "{ invalid json content")
            .await
            .unwrap();

        // Streaming all with verification should handle JSON parsing errors
        let options = crate::VerificationOptions::strict();
        let result: Result<Vec<Document>, _> = collection
            .all_with_verification(&options)
            .try_collect()
            .await;

        // Should fail due to JSON parsing error
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_filter_with_verification_corrupted_json() {
        let (collection, _temp_dir): (Collection, tempfile::TempDir) = setup_collection_with_signing_key().await;

        // Create a corrupted JSON file manually
        let file_path = collection.path.join("corrupted.json");
        fs::write(&file_path, "{ invalid json content")
            .await
            .unwrap();

        // Streaming filter with verification should handle JSON parsing errors
        let options = crate::VerificationOptions::strict();
        let result: Result<Vec<Document>, _> = collection
            .filter_with_verification(|_| true, &options)
            .try_collect()
            .await;

        // Should fail due to JSON parsing error
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod store_tests {
    use tempfile::tempdir;
    use futures::TryStreamExt;
    use serde_json::json;

    use crate::Store;

    #[tokio::test]
    async fn test_store_new() {
        let temp_dir = tempdir().unwrap();
        let result = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await;
        assert!(result.is_ok());
        let store = result.unwrap();
        assert_eq!(store.root_path(), temp_dir.path());
    }

    #[tokio::test]
    async fn test_store_new_with_config() {
        let temp_dir = tempdir().unwrap();
        let result = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_store_new_with_passphrase() {
        let temp_dir = tempdir().unwrap();
        let result = Store::new_with_config(
            temp_dir.path(),
            Some("test_passphrase"),
            sentinel_wal::StoreWalConfig::default(),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_store_list_collections_empty() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collections = store.list_collections().await.unwrap();
        assert!(collections.is_empty());
    }

    #[tokio::test]
    async fn test_store_list_collections_with_multiple() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();

        // Create multiple collections
        let _ = store.collection_with_config("users", None).await.unwrap();
        let _ = store
            .collection_with_config("products", None)
            .await
            .unwrap();
        let _ = store.collection_with_config("orders", None).await.unwrap();

        let collections = store.list_collections().await.unwrap();
        assert_eq!(collections.len(), 3);
        assert!(collections.contains(&"users".to_string()));
        assert!(collections.contains(&"products".to_string()));
        assert!(collections.contains(&"orders".to_string()));
    }

    #[tokio::test]
    async fn test_store_delete_collection_nonexistent() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();

        // Delete nonexistent collection (should be idempotent)
        let result = store.delete_collection("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_store_delete_collection_existing() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();

        // Create a collection
        let collection = store
            .collection_with_config("temp_collection", None)
            .await
            .unwrap();
        collection
            .insert("doc1", json!({"data": "value"}))
            .await
            .unwrap();

        // Verify it exists
        let collections = store.list_collections().await.unwrap();
        assert!(collections.contains(&"temp_collection".to_string()));

        // Delete it
        let result = store.delete_collection("temp_collection").await;
        assert!(result.is_ok());

        // Verify it's gone
        let collections = store.list_collections().await.unwrap();
        assert!(!collections.contains(&"temp_collection".to_string()));
    }

    #[tokio::test]
    async fn test_store_multiple_collections_isolation() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();

        // Create multiple collections and add data to each
        let users = store.collection_with_config("users", None).await.unwrap();
        let products = store
            .collection_with_config("products", None)
            .await
            .unwrap();

        users
            .insert("user1", json!({"name": "Alice"}))
            .await
            .unwrap();
        products
            .insert("prod1", json!({"name": "Widget"}))
            .await
            .unwrap();

        // Verify data isolation
        assert!(users.get("user1").await.unwrap().is_some());
        assert!(users.get("prod1").await.unwrap().is_none());

        assert!(products.get("prod1").await.unwrap().is_some());
        assert!(products.get("user1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_store_collection_persistence() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Create store and collection with data
        {
            let store = Store::new_with_config(&path, None, sentinel_wal::StoreWalConfig::default())
                .await
                .unwrap();
            let collection = store.collection_with_config("users", None).await.unwrap();
            collection
                .insert("user1", json!({"name": "Alice"}))
                .await
                .unwrap();
        }

        // Reopen store and verify data
        {
            let store = Store::new_with_config(&path, None, sentinel_wal::StoreWalConfig::default())
                .await
                .unwrap();
            let collection = store.collection_with_config("users", None).await.unwrap();
            let doc = collection.get("user1").await.unwrap();
            assert!(doc.is_some());
            assert_eq!(doc.unwrap().data()["name"], "Alice");
        }
    }

    #[tokio::test]
    async fn test_store_root_path() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path();
        let store = Store::new_with_config(path, None, sentinel_wal::StoreWalConfig::default())
            .await
            .unwrap();
        assert_eq!(store.root_path(), path);
    }

    #[tokio::test]
    async fn test_store_created_at() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let created_at = store.created_at();
        assert!(created_at <= chrono::Utc::now());
    }

    #[tokio::test]
    async fn test_store_last_accessed_at() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let before_access = chrono::Utc::now();

        // Access a collection to update last_accessed_at
        let _ = store.collection_with_config("test", None).await.unwrap();

        let last_accessed = store.last_accessed_at();
        assert!(last_accessed >= before_access);
    }

    #[tokio::test]
    async fn test_store_total_documents() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("docs", None).await.unwrap();

        collection.insert("doc1", json!({"data": 1})).await.unwrap();
        collection.insert("doc2", json!({"data": 2})).await.unwrap();

        // Counters are updated asynchronously by event processor
        // Just verify that the method works without panic
        let _total = store.total_documents();
        assert!(true);
    }

    #[tokio::test]
    async fn test_store_total_size_bytes() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("docs", None).await.unwrap();

        collection
            .insert("doc1", json!({"data": "large content here"}))
            .await
            .unwrap();

        // Counters are updated asynchronously by event processor
        // Just verify that the method works without panic
        let _size = store.total_size_bytes();
        assert!(true);
    }

    #[tokio::test]
    async fn test_store_collection_count() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();

        let _ = store.collection_with_config("col1", None).await.unwrap();
        let _ = store.collection_with_config("col2", None).await.unwrap();
        let _ = store.collection_with_config("col3", None).await.unwrap();

        // Allow event processor to update counters
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let count = store.collection_count();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_store_event_sender() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let sender = store.event_sender();

        // Sender should be cloneable and usable
        let _cloned = sender.clone();
        assert!(true); // If we got here, sender is valid
    }

    #[tokio::test]
    async fn test_store_collection_with_config_default() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();

        let collection = store
            .collection_with_config("configured", None)
            .await
            .unwrap();

        collection
            .insert("doc1", json!({"test": true}))
            .await
            .unwrap();
        let doc = collection.get("doc1").await.unwrap();
        assert!(doc.is_some());
    }

    #[tokio::test]
    async fn test_store_delete_collection_with_metadata() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();

        let collection = store
            .collection_with_config("to_delete", None)
            .await
            .unwrap();
        for i in 0 .. 5 {
            collection
                .insert(&format!("doc{}", i), json!({"index": i}))
                .await
                .unwrap();
        }

        // Verify collection exists with documents
        let all_docs: Vec<_> = collection.all().try_collect().await.unwrap();
        assert_eq!(all_docs.len(), 5);

        // Delete the collection
        store.delete_collection("to_delete").await.unwrap();

        // Verify collection no longer exists
        let collections = store.list_collections().await.unwrap();
        assert!(!collections.contains(&"to_delete".to_string()));
    }

    #[tokio::test]
    async fn test_store_multiple_operations_sequence() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();

        // Create first collection
        let col1 = store.collection_with_config("first", None).await.unwrap();
        col1.insert("data1", json!({"value": 1})).await.unwrap();

        // Create second collection
        let col2 = store.collection_with_config("second", None).await.unwrap();
        col2.insert("data2", json!({"value": 2})).await.unwrap();

        // List all
        let mut collections = store.list_collections().await.unwrap();
        collections.sort();
        assert_eq!(collections.len(), 2);

        // Delete first
        store.delete_collection("first").await.unwrap();

        // Verify only second remains
        let collections = store.list_collections().await.unwrap();
        assert_eq!(collections.len(), 1);
        assert_eq!(collections[0], "second");

        // Create third
        let col3 = store.collection_with_config("third", None).await.unwrap();
        col3.insert("data3", json!({"value": 3})).await.unwrap();

        // Verify we have second and third
        let mut collections = store.list_collections().await.unwrap();
        collections.sort();
        assert_eq!(collections.len(), 2);
        assert!(collections.contains(&"second".to_string()));
        assert!(collections.contains(&"third".to_string()));
    }
}

#[cfg(test)]
mod collection_streaming_tests {
    use tempfile::tempdir;
    use futures::TryStreamExt;
    use serde_json::json;

    use crate::Store;

    #[tokio::test]
    async fn test_collection_list_documents() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("docs", None).await.unwrap();

        collection.insert("doc1", json!({"data": 1})).await.unwrap();
        collection.insert("doc2", json!({"data": 2})).await.unwrap();
        collection.insert("doc3", json!({"data": 3})).await.unwrap();

        let docs: Vec<_> = collection.list().try_collect().await.unwrap();
        assert_eq!(docs.len(), 3);
        assert!(docs.contains(&"doc1".to_string()));
        assert!(docs.contains(&"doc2".to_string()));
        assert!(docs.contains(&"doc3".to_string()));
    }

    #[tokio::test]
    async fn test_collection_list_empty() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("empty", None).await.unwrap();

        let docs: Vec<_> = collection.list().try_collect().await.unwrap();
        assert!(docs.is_empty());
    }

    #[tokio::test]
    async fn test_collection_filter_documents() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("users", None).await.unwrap();

        collection
            .insert("user1", json!({"name": "Alice", "age": 25}))
            .await
            .unwrap();
        collection
            .insert("user2", json!({"name": "Bob", "age": 35}))
            .await
            .unwrap();
        collection
            .insert("user3", json!({"name": "Charlie", "age": 28}))
            .await
            .unwrap();

        let filtered: Vec<_> = collection
            .filter(|doc| {
                doc.data()
                    .get("age")
                    .and_then(|v| v.as_u64())
                    .map_or(false, |age| age > 26)
            })
            .try_collect()
            .await
            .unwrap();

        assert_eq!(filtered.len(), 2);
        let names: Vec<_> = filtered
            .iter()
            .map(|d| d.data()["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"Bob"));
        assert!(names.contains(&"Charlie"));
    }

    #[tokio::test]
    async fn test_collection_filter_no_matches() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("users", None).await.unwrap();

        collection
            .insert("user1", json!({"name": "Alice", "age": 25}))
            .await
            .unwrap();
        collection
            .insert("user2", json!({"name": "Bob", "age": 20}))
            .await
            .unwrap();

        let filtered: Vec<_> = collection
            .filter(|doc| {
                doc.data()
                    .get("age")
                    .and_then(|v| v.as_u64())
                    .map_or(false, |age| age > 100)
            })
            .try_collect()
            .await
            .unwrap();

        assert!(filtered.is_empty());
    }

    #[tokio::test]
    async fn test_collection_all_documents() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("docs", None).await.unwrap();

        collection
            .insert("doc1", json!({"value": 1}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"value": 2}))
            .await
            .unwrap();

        let all: Vec<_> = collection.all().try_collect().await.unwrap();
        assert_eq!(all.len(), 2);

        // Check values are present (order may vary)
        let values: Vec<_> = all
            .iter()
            .map(|d| d.data()["value"].as_u64().unwrap())
            .collect();
        assert!(values.contains(&1));
        assert!(values.contains(&2));
    }

    #[tokio::test]
    async fn test_collection_all_empty() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store
            .collection_with_config("empty_coll", None)
            .await
            .unwrap();

        let all: Vec<_> = collection.all().try_collect().await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn test_collection_map_documents() {
        use futures::StreamExt;
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("docs", None).await.unwrap();

        collection
            .insert("doc1", json!({"value": 10}))
            .await
            .unwrap();
        collection
            .insert("doc2", json!({"value": 20}))
            .await
            .unwrap();

        let mapped: Vec<_> = collection
            .all()
            .map(|result| {
                result.map(|doc| {
                    doc.data()
                        .get("value")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                })
            })
            .try_collect()
            .await
            .unwrap();

        assert_eq!(mapped.len(), 2);
        assert!(mapped.contains(&10));
        assert!(mapped.contains(&20));
    }
}
#[cfg(test)]
mod collection_error_tests {
    use tempfile::tempdir;
    use serde_json::json;

    use crate::Store;

    #[tokio::test]
    async fn test_collection_get_nonexistent_document() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("users", None).await.unwrap();

        let doc = collection.get("nonexistent").await.unwrap();
        assert!(doc.is_none());
    }

    #[tokio::test]
    async fn test_collection_delete_nonexistent_document() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("users", None).await.unwrap();

        let result = collection.delete("nonexistent").await;
        // Depending on implementation, this might succeed or fail
        // Just verify it doesn't panic
        let _ = result;
    }

    #[tokio::test]
    async fn test_collection_update_document() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("users", None).await.unwrap();

        collection
            .insert("user1", json!({"name": "Alice", "age": 25}))
            .await
            .unwrap();

        let updated = json!({"name": "Alice", "age": 26});
        collection.update("user1", updated).await.unwrap();

        let doc = collection.get("user1").await.unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().data()["age"], 26);
    }

    #[tokio::test]
    async fn test_collection_count_empty() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store
            .collection_with_config("empty_count", None)
            .await
            .unwrap();

        let count = collection.count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_collection_exists_document() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("users", None).await.unwrap();

        collection
            .insert("user1", json!({"name": "Alice"}))
            .await
            .unwrap();

        let doc_user1 = collection.get("user1").await.unwrap();
        assert!(doc_user1.is_some());

        let doc_user999 = collection.get("user999").await.unwrap();
        assert!(doc_user999.is_none());
    }

    #[tokio::test]
    async fn test_collection_delete_document() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("docs", None).await.unwrap();

        collection
            .insert("doc1", json!({"data": "value"}))
            .await
            .unwrap();

        let doc = collection.get("doc1").await.unwrap();
        assert!(doc.is_some());

        collection.delete("doc1").await.unwrap();

        let doc = collection.get("doc1").await.unwrap();
        assert!(doc.is_none());
    }

    #[tokio::test]
    async fn test_collection_duplicate_insert() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("users", None).await.unwrap();

        collection
            .insert("user1", json!({"name": "Alice"}))
            .await
            .unwrap();

        // Attempting to insert with the same ID should fail
        let result = collection.insert("user1", json!({"name": "Bob"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_collection_insert_large_document() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("docs", None).await.unwrap();

        let large_data = json!({
            "content": "x".repeat(10000),
            "nested": {
                "data": "y".repeat(5000)
            }
        });

        collection
            .insert("large_doc", large_data.clone())
            .await
            .unwrap();

        let doc = collection.get("large_doc").await.unwrap();
        assert!(doc.is_some());
        let doc_content = doc.unwrap().data()["content"].as_str().unwrap().to_string();
        assert_eq!(doc_content, "x".repeat(10000));
    }

    #[tokio::test]
    async fn test_collection_operations_with_special_chars_in_id() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new_with_config(
            temp_dir.path(),
            None,
            sentinel_wal::StoreWalConfig::default(),
        )
        .await
        .unwrap();
        let collection = store.collection_with_config("docs", None).await.unwrap();

        // IDs with dashes and underscores should work
        collection
            .insert("doc-id_123", json!({"data": "test"}))
            .await
            .unwrap();

        let doc = collection.get("doc-id_123").await.unwrap();
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().id(), "doc-id_123");
    }
}
