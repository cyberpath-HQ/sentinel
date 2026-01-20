#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::{SentinelError, Store};

    #[tokio::test]
    async fn test_store_new_creates_directory() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path().join("store");

        let _store = Store::new(&store_path, None).await.unwrap();
        assert!(store_path.exists());
        assert!(store_path.is_dir());
    }

    #[tokio::test]
    async fn test_store_new_with_existing_directory() {
        let temp_dir = tempdir().unwrap();
        let store_path = temp_dir.path();

        // Directory already exists
        let _store = Store::new(&store_path, None).await.unwrap();
        assert!(store_path.exists());
    }

    #[tokio::test]
    async fn test_store_collection_creates_subdirectory() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        let collection = store.collection("users").await.unwrap();
        assert!(collection.path.exists());
        assert!(collection.path.is_dir());
        assert_eq!(collection.name(), "users");
    }

    #[tokio::test]
    async fn test_store_collection_with_valid_special_characters() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Test valid names with underscores, hyphens, and dots
        let collection = store.collection("user_data-123").await.unwrap();
        assert!(collection.path.exists());
        assert_eq!(collection.name(), "user_data-123");

        let collection2 = store.collection("test.collection").await.unwrap();
        assert!(collection2.path.exists());
        assert_eq!(collection2.name(), "test.collection");

        let collection3 = store.collection("data_2024-v1.0").await.unwrap();
        assert!(collection3.path.exists());
        assert_eq!(collection3.name(), "data_2024-v1.0");
    }

    #[tokio::test]
    async fn test_store_collection_multiple_calls() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        let coll1 = store.collection("users").await.unwrap();
        let coll2 = store.collection("users").await.unwrap();

        assert_eq!(coll1.name(), coll2.name());
        assert_eq!(coll1.path, coll2.path);
    }

    #[tokio::test]
    async fn test_store_collection_invalid_empty_name() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        let result = store.collection("").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));
    }

    #[tokio::test]
    async fn test_store_collection_invalid_path_separator() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Forward slash
        let result = store.collection("path/traversal").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));

        // Backslash
        let result = store.collection("path\\traversal").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));
    }

    #[tokio::test]
    async fn test_store_collection_invalid_hidden_name() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        let result = store.collection(".hidden").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));
    }

    #[tokio::test]
    async fn test_store_collection_invalid_windows_reserved_names() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        let reserved_names = vec!["CON", "PRN", "AUX", "NUL", "COM1", "LPT1"];
        for name in reserved_names {
            let result = store.collection(name).await;
            assert!(result.is_err(), "Expected '{}' to be invalid", name);
            assert!(matches!(
                result.unwrap_err(),
                SentinelError::InvalidCollectionName { .. }
            ));

            // Test lowercase version
            let result = store.collection(&name.to_lowercase()).await;
            assert!(
                result.is_err(),
                "Expected '{}' to be invalid",
                name.to_lowercase()
            );
            assert!(matches!(
                result.unwrap_err(),
                SentinelError::InvalidCollectionName { .. }
            ));
        }
    }

    #[tokio::test]
    async fn test_store_collection_invalid_control_characters() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Test null byte
        let result = store.collection("test\0name").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));

        // Test other control characters
        let result = store.collection("test\x01name").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));
    }

    #[tokio::test]
    async fn test_store_collection_invalid_special_characters() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        let invalid_chars = vec!["<", ">", ":", "\"", "|", "?", "*"];
        for ch in invalid_chars {
            let name = format!("test{}name", ch);
            let result = store.collection(&name).await;
            assert!(result.is_err(), "Expected name with '{}' to be invalid", ch);
            assert!(matches!(
                result.unwrap_err(),
                SentinelError::InvalidCollectionName { .. }
            ));
        }
    }

    #[tokio::test]
    async fn test_store_collection_invalid_trailing_dot_or_space() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Trailing dot
        let result = store.collection("test.").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));

        // Trailing space
        let result = store.collection("test ").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SentinelError::InvalidCollectionName { .. }
        ));
    }

    #[tokio::test]
    async fn test_store_collection_valid_edge_cases() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Single character
        let collection = store.collection("a").await.unwrap();
        assert_eq!(collection.name(), "a");

        // Numbers only
        let collection = store.collection("123").await.unwrap();
        assert_eq!(collection.name(), "123");

        // Max length typical name
        let long_name = "a".repeat(255);
        let collection = store.collection(&long_name).await.unwrap();
        assert_eq!(collection.name(), long_name);
    }

    #[tokio::test]
    async fn test_store_new_with_passphrase() {
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();
        // Should have created signing key
        assert!(store.signing_key.is_some());
    }

    #[tokio::test]
    async fn test_store_new_with_passphrase_load_existing() {
        let temp_dir = tempdir().unwrap();
        // Create first store with passphrase
        let store1 = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();
        let key1 = store1.signing_key.as_ref().unwrap().clone();

        // Create second store with same passphrase, should load existing key
        let store2 = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();
        let key2 = store2.signing_key.as_ref().unwrap().clone();

        // Should be the same key
        assert_eq!(key1.to_bytes(), key2.to_bytes());
    }

    #[tokio::test]
    async fn test_store_new_with_corrupted_keys() {
        let temp_dir = tempdir().unwrap();
        // First create a store with passphrase to generate keys
        let _store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();

        // Now corrupt the .keys collection by inserting a document with missing fields
        let store2 = Store::new(temp_dir.path(), None).await.unwrap();
        let keys_coll = store2.collection(".keys").await.unwrap();
        // Insert corrupted document
        let corrupted_data = serde_json::json!({
            "salt": "invalid_salt",
            // missing "encrypted"
        });
        keys_coll
            .insert("signing_key", corrupted_data)
            .await
            .unwrap();

        // Now try to create a new store with passphrase, should fail due to corruption
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_new_with_invalid_salt_hex() {
        let temp_dir = tempdir().unwrap();
        // First create a store with passphrase to generate keys
        let _store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();

        // Corrupt the salt to invalid hex
        let store2 = Store::new(temp_dir.path(), None).await.unwrap();
        let keys_coll = store2.collection(".keys").await.unwrap();
        let doc = keys_coll
            .get_with_verification("signing_key", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        let mut data = doc.data().clone();
        data["salt"] = serde_json::Value::String("invalid_hex".to_string());
        keys_coll.insert("signing_key", data).await.unwrap();

        // Try to load
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_new_with_invalid_encrypted_length() {
        let temp_dir = tempdir().unwrap();
        // First create a store with passphrase to generate keys
        let _store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();

        // Corrupt the encrypted to short
        let store2 = Store::new(temp_dir.path(), None).await.unwrap();
        let keys_coll = store2.collection(".keys").await.unwrap();
        let doc = keys_coll
            .get_with_verification("signing_key", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        let mut data = doc.data().clone();
        data["encrypted"] = serde_json::Value::String(hex::encode(&[0u8; 10])); // short
        keys_coll.insert("signing_key", data).await.unwrap();

        // Try to load
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_new_with_corrupted_keys_missing_salt() {
        let temp_dir = tempdir().unwrap();
        // First create a store with passphrase to generate keys
        let _store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();

        // Now corrupt the .keys collection by inserting a document with missing salt
        let store2 = Store::new(temp_dir.path(), None).await.unwrap();
        let keys_coll = store2.collection(".keys").await.unwrap();
        // Insert corrupted document
        let corrupted_data = serde_json::json!({
            "encrypted": "some_encrypted_data"
            // missing "salt"
        });
        keys_coll
            .insert("signing_key", corrupted_data)
            .await
            .unwrap();

        // Now try to create a new store with passphrase, should fail due to missing salt
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_new_with_corrupted_keys_invalid_salt_hex() {
        let temp_dir = tempdir().unwrap();
        // First create a store with passphrase to generate keys
        let _store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();

        // Now corrupt the .keys collection by inserting a document with invalid salt hex
        let store2 = Store::new(temp_dir.path(), None).await.unwrap();
        let keys_coll = store2.collection(".keys").await.unwrap();
        // Insert corrupted document
        let corrupted_data = serde_json::json!({
            "encrypted": "some_encrypted_data",
            "salt": "invalid_hex_salt"
        });
        keys_coll
            .insert("signing_key", corrupted_data)
            .await
            .unwrap();

        // Now try to create a new store with passphrase, should fail due to invalid salt hex
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_new_with_invalid_key_length() {
        // Test line 154-161: invalid key length error
        let temp_dir = tempdir().unwrap();
        // First create a store with passphrase to generate keys
        let _store = Store::new(temp_dir.path(), Some("test_passphrase"))
            .await
            .unwrap();

        // Now corrupt the .keys collection by modifying the encrypted data to have wrong length
        let store2 = Store::new(temp_dir.path(), None).await.unwrap();
        let keys_coll = store2.collection(".keys").await.unwrap();

        // Get the existing document to extract the salt
        let existing_doc = keys_coll
            .get_with_verification("signing_key", &crate::VerificationOptions::disabled())
            .await
            .unwrap()
            .unwrap();
        let salt = existing_doc.data()["salt"].as_str().unwrap();

        // Create encrypted data that will decrypt to wrong length
        let encryption_key =
            sentinel_crypto::derive_key_from_passphrase_with_salt("test_passphrase", &hex::decode(salt).unwrap())
                .await
                .unwrap();
        let wrong_length_bytes = vec![0u8; 16]; // 16 bytes instead of 32
        let encrypted = sentinel_crypto::encrypt_data(&wrong_length_bytes, &encryption_key)
            .await
            .unwrap();

        let corrupted_data = serde_json::json!({
            "encrypted": encrypted,
            "salt": salt
        });
        keys_coll
            .insert("signing_key", corrupted_data)
            .await
            .unwrap();

        // Now try to create a new store with passphrase, should fail due to invalid key length
        let result = Store::new(temp_dir.path(), Some("test_passphrase")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_new_creates_root_directory() {
        // Test line 110-117: creating root directory
        let temp_dir = tempdir().unwrap();
        let new_path = temp_dir.path().join("new_store");

        // Ensure path doesn't exist
        assert!(!tokio::fs::metadata(&new_path).await.is_ok());

        // Create store, should create the directory
        let result = Store::new(&new_path, None).await;
        assert!(result.is_ok());

        // Verify directory was created
        assert!(tokio::fs::metadata(&new_path).await.unwrap().is_dir());
    }

    #[tokio::test]
    async fn test_delete_collection_non_existent() {
        // Test lines 304-306: Deleting non-existent collection
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Delete collection that doesn't exist should succeed
        let result = store.delete_collection("non_existent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_collection_success() {
        // Test lines 310-312, 315-316: Successful collection deletion
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Create a collection
        let _collection = store.collection("test_delete").await.unwrap();

        // Verify it exists
        let collections = store.list_collections().await.unwrap();
        assert!(collections.contains(&"test_delete".to_string()));

        // Delete it
        store.delete_collection("test_delete").await.unwrap();

        // Verify it's gone
        let collections = store.list_collections().await.unwrap();
        assert!(!collections.contains(&"test_delete".to_string()));
    }

    #[tokio::test]
    async fn test_list_collections_creates_data_dir() {
        // Test lines 352-354: list_collections creates data directory if needed
        let temp_dir = tempdir().unwrap();
        let new_path = temp_dir.path().join("new_store");
        let store = Store::new(&new_path, None).await.unwrap();

        // Data dir should be created when listing
        let collections = store.list_collections().await.unwrap();
        assert!(collections.is_empty());

        // Verify data directory exists
        let data_path = new_path.join("data");
        assert!(tokio::fs::metadata(&data_path).await.unwrap().is_dir());
    }

    #[tokio::test]
    async fn test_list_collections_with_entries() {
        // Test lines 363-366, 368-371, 376-377: Reading directory entries
        let temp_dir = tempdir().unwrap();
        let store = Store::new(temp_dir.path(), None).await.unwrap();

        // Create multiple collections
        let _c1 = store.collection("collection1").await.unwrap();
        let _c2 = store.collection("collection2").await.unwrap();
        let _c3 = store.collection("collection3").await.unwrap();

        // List and verify
        let collections = store.list_collections().await.unwrap();
        assert_eq!(collections.len(), 3);
        assert!(collections.contains(&"collection1".to_string()));
        assert!(collections.contains(&"collection2".to_string()));
        assert!(collections.contains(&"collection3".to_string()));
    }
}
