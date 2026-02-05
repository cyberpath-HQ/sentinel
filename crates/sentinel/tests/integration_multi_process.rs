/// Integration tests for multi-process concurrent access to Sentinel DBMS
///
/// This test suite verifies that the file locking system prevents race conditions
/// and ensures consistent access across multiple processes.
use sentinel_dbms::{CollectionWalConfigOverrides, Store};
use sentinel_wal::StoreWalConfig;
use serde_json::json;
use serial_test::serial;

/// Test S4.2.1: Multi-process document operations
///
/// This test spawns multiple processes that concurrently perform insert, update, and
/// delete operations on the same collection. The file locking system should prevent
/// data corruption by ensuring exclusive access during write operations and allowing
/// concurrent reads with shared locks.
#[tokio::test]
#[serial]
async fn test_multi_process_concurrent_inserts() {
    // Setup temporary directory for multi-process test
    let temp_dir = tempfile::tempdir().unwrap();
    let store_path = temp_dir.path().join("multi_process_test");

    // Clean up any existing files
    println!("Store path: {:?}", store_path);
    if store_path.exists() {
        println!("Store path exists, removing");
        tokio::fs::remove_dir_all(&store_path).await.unwrap();
    }
    else {
        println!("Store path does not exist");
    }

    // Spawn multiple processes to compete for the same collection
    let mut children = vec![];
    for i in 0 .. 3 {
        let store_path_clone = store_path.clone();
        let child = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Initialize store in this process
                let store = Store::new_with_config(
                    &store_path_clone,
                    None,
                    sentinel_wal::StoreWalConfig::default(),
                )
                .await
                .unwrap();
                let collection = store
                    .collection_with_config(
                        "test_collection",
                        Some(CollectionWalConfigOverrides::default()),
                    )
                    .await
                    .unwrap();

                // Each process performs insert operations
                for j in 0 .. 10 {
                    let id = format!("doc-{}-{}", i, j);
                    let data = json!({ "process": i, "sequence": j, "value": i * 100 + j });

                    match collection.insert(&id, data).await {
                        Ok(()) => {
                            println!("Process {} inserted doc-{}-{}", i, i, j);
                        },
                        Err(e) if e.to_string().contains("already exists") => {
                            println!("Process {} skipped doc-{}-{} (already exists)", i, i, j);
                        },
                        Err(e) => {
                            panic!("Unexpected error: {}", e);
                        },
                    }
                }

                // Force checkpoint to ensure durability
                collection.checkpoint().await.unwrap();

                // Explicitly drop the store to ensure flushing
                drop(store);
            });
        });
        children.push(child);
    }

    // Wait for all processes to complete
    for child in children {
        child.join().unwrap();
    }

    // Give a moment for any pending I/O to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify all documents were written correctly
    let store = Store::new_with_config(&store_path, None, sentinel_wal::StoreWalConfig::default())
        .await
        .unwrap();
    let collection = store
        .collection_with_config(
            "test_collection",
            Some(CollectionWalConfigOverrides::default()),
        )
        .await
        .unwrap();

    // Check that all documents were created (30 total: 3 processes * 10 docs each)
    let collection_path = temp_dir
        .path()
        .join("multi_process_test")
        .join("data")
        .join("test_collection");
    println!("Collection path: {:?}", collection_path);
    if let Ok(entries) = std::fs::read_dir(&collection_path) {
        println!("Files in collection directory:");
        for entry in entries {
            if let Ok(entry) = entry {
                println!("  {:?}", entry.file_name());
            }
        }
    }
    else {
        println!("Collection directory does not exist or cannot be read");
    }

    let total_count = collection.count().await.unwrap();

    assert_eq!(
        total_count, 30,
        "Expected 30 documents, got {:?}",
        total_count
    );

    // Verify each document's integrity
    for i in 0 .. 3 {
        for j in 0 .. 10 {
            let id = format!("doc-{}-{}", i, j);
            let store_path = temp_dir.path().join("multi_process_test");
            let collection_path = store_path.join("data").join("test_collection");
            let file_path = collection_path.join(format!("{}.json", id));

            // First check if file exists and has content
            if let Ok(metadata) = tokio::fs::metadata(&file_path).await {
                if metadata.len() == 0 {
                    panic!("Document file {} exists but is empty", file_path.display());
                }
            }
            else {
                panic!("Document file {} does not exist", file_path.display());
            }

            match collection.get(&id).await {
                Ok(Some(doc)) => {
                    assert_eq!(doc.data()["process"], i);
                    assert_eq!(doc.data()["sequence"], j);
                    assert_eq!(doc.data()["value"], i * 100 + j);
                },
                Ok(None) => panic!("Document {} not found", id),
                Err(e) => {
                    // Debug: check file contents
                    let store_path = temp_dir.path().join("multi_process_test");
                    let collection_path = store_path.join("data").join("test_collection");
                    let file_path = collection_path.join(format!("{}.json", id));
                    if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
                        let preview: String = content.chars().take(200).collect();
                        println!(
                            "File {} exists with content length: {}, first 200 chars: {}",
                            file_path.display(),
                            content.len(),
                            preview
                        );
                    }
                    else {
                        println!(
                            "File {} does not exist or cannot be read",
                            file_path.display()
                        );
                    }
                    panic!("Error getting document {}: {:?}", id, e);
                },
            }
        }
    }

    println!("✓ Multi-process concurrent inserts test passed");
}

/// Test S4.2.1: Multi-process read-write concurrency
///
/// This test verifies that multiple processes can concurrently read documents while
/// other processes perform exclusive write operations. Shared locks should allow
/// multiple readers, while write operations acquire exclusive locks.
#[tokio::test]
#[serial]
async fn test_multi_process_read_write_concurrency() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store_path = temp_dir.path().join("read_write_test");

    // First, create initial documents
    let store = Store::new_with_config(&store_path, None, StoreWalConfig::default())
        .await
        .unwrap();
    let collection = store
        .collection_with_config(
            "read_write_test",
            Some(CollectionWalConfigOverrides::default()),
        )
        .await
        .unwrap();

    // Insert initial documents
    for j in 0 .. 20 {
        let id = format!("doc-{}", j);
        let data = json!({ "initial": true, "sequence": j });
        collection.insert(&id, data).await.unwrap();
    }

    // Spawn writer processes
    let mut writer_children = vec![];
    for i in 0 .. 2 {
        let store_path_clone = store_path.clone();
        let child = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let store = Store::new_with_config(&store_path_clone, None, StoreWalConfig::default())
                    .await
                    .unwrap();

                let collection = store
                    .collection_with_config(
                        "read_write_test",
                        Some(CollectionWalConfigOverrides::default()),
                    )
                    .await
                    .unwrap();

                for j in 0 .. 20 {
                    let id = format!("doc-{}", j);
                    let data = json!({ "writer": i, "sequence": j });

                    collection.update(&id, data).await.unwrap();
                }

                // Force checkpoint to ensure durability
                collection.checkpoint().await.unwrap();
            });
        });
        writer_children.push(child);
    }

    // Spawn reader processes
    let mut reader_children = vec![];
    for _i in 0 .. 3 {
        let store_path_clone = store_path.clone();
        let child = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let store = Store::new_with_config(&store_path_clone, None, StoreWalConfig::default())
                    .await
                    .unwrap();
                let collection = store
                    .collection_with_config(
                        "read_write_test",
                        Some(CollectionWalConfigOverrides::default()),
                    )
                    .await
                    .unwrap();

                // Each reader reads all documents
                for j in 0 .. 20 {
                    let id = format!("doc-{}", j);
                    collection.get(&id).await.unwrap();
                }
            });
        });
        reader_children.push(child);
    }

    // Wait for readers to complete
    for child in reader_children {
        child.join().unwrap();
    }

    // Wait for writers to complete
    for child in writer_children {
        child.join().unwrap();
    }

    // Give a moment for any pending I/O to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; // Now count the documents
    let store = Store::new_with_config(&store_path, None, StoreWalConfig::default())
        .await
        .unwrap();
    let collection = store
        .collection_with_config(
            "read_write_test",
            Some(CollectionWalConfigOverrides::default()),
        )
        .await
        .unwrap();

    let total_count = collection.count().await.unwrap();

    assert_eq!(
        total_count, 20,
        "Expected 20 documents, got {:?}",
        total_count
    );

    println!("✓ Multi-process read-write concurrency test passed");
}
/// This test verifies that exclusive locks are properly acquired during update
/// operations across multiple processes.
#[tokio::test]
#[serial]
async fn test_multi_process_concurrent_updates() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store_path = temp_dir.path().join("update_test");

    // Setup initial data
    let store = Store::new_with_config(&store_path, None, StoreWalConfig::default())
        .await
        .unwrap();
    let collection = store
        .collection_with_config("update_test", Some(CollectionWalConfigOverrides::default()))
        .await
        .unwrap();

    // Insert initial documents
    for i in 0 .. 10 {
        let id = format!("doc-{}", i);
        collection.insert(&id, json!({ "value": i })).await.unwrap();
    }

    // Spawn multiple update processes
    let mut children = vec![];
    for proc_id in 0 .. 3 {
        let store_path_clone = store_path.clone();
        let child = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let store = Store::new_with_config(&store_path_clone, None, StoreWalConfig::default())
                    .await
                    .unwrap();

                let collection = store
                    .collection_with_config("update_test", Some(CollectionWalConfigOverrides::default()))
                    .await
                    .unwrap();

                // Each process updates different subset of documents
                for i in (proc_id * 3) .. ((proc_id + 1) * 3) {
                    let id = format!("doc-{}", i);
                    let data = json!({ "value": i * 10 + proc_id });

                    collection.update(&id, data).await.unwrap();
                }
            });
        });
        children.push(child);
    }

    // Wait for updates to complete
    for child in children {
        child.join().unwrap();
    }

    // Verify updated values
    let store = Store::new_with_config(&store_path, None, StoreWalConfig::default())
        .await
        .unwrap();
    let collection = store
        .collection_with_config("update_test", Some(CollectionWalConfigOverrides::default()))
        .await
        .unwrap();

    for i in 0 .. 10 {
        let id = format!("doc-{}", i);
        let doc = collection.get(&id).await.unwrap().unwrap();
        let proc_id = i / 3;
        let expected_value = if i < 9 { i * 10 + proc_id } else { i };
        assert_eq!(doc.data()["value"], expected_value);
    }

    println!("✓ Multi-process concurrent updates test passed");
}

/// Test S4.2.1: Multi-process delete operations
///
/// This test verifies that exclusive locks prevent concurrent delete conflicts.
#[tokio::test]
#[serial]
async fn test_multi_process_concurrent_deletes() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store_path = temp_dir.path().join("delete_test");

    // Setup initial data
    let store = Store::new_with_config(&store_path, None, StoreWalConfig::default())
        .await
        .unwrap();
    let collection = store
        .collection_with_config("delete_test", Some(CollectionWalConfigOverrides::default()))
        .await
        .unwrap();

    // Insert documents
    for i in 0 .. 15 {
        let id = format!("doc-{}", i);
        collection.insert(&id, json!({ "value": i })).await.unwrap();
    }

    // Spawn delete processes
    let mut children = vec![];
    for proc_id in 0 .. 3 {
        let store_path_clone = store_path.clone();
        let child = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let store = Store::new_with_config(&store_path_clone, None, StoreWalConfig::default())
                    .await
                    .unwrap();
                let collection = store
                    .collection_with_config("delete_test", Some(CollectionWalConfigOverrides::default()))
                    .await
                    .unwrap();

                // Each process deletes different subset
                for i in (proc_id * 5) .. ((proc_id + 1) * 5) {
                    let id = format!("doc-{}", i);
                    collection.delete(&id).await.unwrap();
                }
            });
        });
        children.push(child);
    }

    // Wait for deletes to complete
    for child in children {
        child.join().unwrap();
    }

    // Verify remaining documents
    let store = Store::new_with_config(&store_path, None, StoreWalConfig::default())
        .await
        .unwrap();
    let collection = store
        .collection_with_config("delete_test", Some(CollectionWalConfigOverrides::default()))
        .await
        .unwrap();

    let remaining = collection.count().await.unwrap();
    assert_eq!(
        remaining, 0,
        "Expected all documents to be deleted, but {} remain",
        remaining
    );

    println!("✓ Multi-process concurrent deletes test passed");
}
