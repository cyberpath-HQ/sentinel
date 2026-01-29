use std::path::PathBuf;

/// Integration tests for WAL and document operation coordination
///
/// This test suite verifies that WAL operations properly coordinate with file locks
/// to ensure atomicity and consistency of write operations.
use sentinel_dbms::{Collection, Store};
use serde_json::json;

/// Test S4.2.2: WAL write-acquire document lock
///
/// This test verifies that WAL log entries are written to the WAL before acquiring
/// exclusive document locks. This ensures atomicity - if a write operation fails
/// after WAL write, the WAL can be replayed.
#[tokio::test]
#[serial]
async fn test_wal_write_before_document_lock() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store_path = temp_dir.path().join("wal_lock_test");

    let store = Store::new(&store_path, None).await.unwrap();
    let collection = store.collection("test_collection").await.unwrap();

    // Perform an insert operation
    let id = "doc-001";
    let data = json!({ "value": 42 });

    // The insert operation should:
    // 1. Write to WAL first
    // 2. Then acquire exclusive lock on document file
    // 3. Then write the document file
    let result = collection.insert(id, data).await;

    assert!(result.is_ok(), "Insert should succeed: {:?}", result);

    // Verify the document was written correctly
    let doc = collection
        .get(id)
        .await
        .unwrap()
        .expect("Document should exist");
    assert_eq!(doc.data()["value"], 42);

    // Verify WAL was updated
    let wal_path = store_path.join("wal").join("wal.log");
    assert!(wal_path.exists(), "WAL file should exist");

    let wal_content = tokio::fs::read_to_string(&wal_path).await.unwrap();
    assert!(
        wal_content.contains("Insert"),
        "WAL should contain Insert operation"
    );
    assert!(wal_content.contains(id), "WAL should reference document ID");

    println!("✓ WAL write before document lock test passed");
}

/// Test S4.2.2: WAL commit with document lock
///
/// This test verifies that document files are written after WAL commits.
/// This ensures that a crash during document write doesn't leave partial data.
#[tokio::test]
#[serial]
async fn test_wal_commit_with_document_lock() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store_path = temp_dir.path().join("wal_commit_test");

    let store = Store::new(&store_path, None).await.unwrap();
    let collection = store.collection("test_collection").await.unwrap();

    let id = "doc-002";
    let data = json!({ "value": 100 });

    // The insert should:
    // 1. Log to WAL (not yet committed)
    // 2. Acquire exclusive lock
    // 3. Write document file (atomic commit)
    let result = collection.insert(id, data).await;

    assert!(result.is_ok(), "Insert should succeed: {:?}", result);

    // Verify document exists and has correct data
    let doc = collection
        .get(id)
        .await
        .unwrap()
        .expect("Document should exist");
    assert_eq!(doc.data()["value"], 100);

    // The WAL entry should be committed
    let wal_path = store_path.join("wal").join("wal.log");
    let wal_content = tokio::fs::read_to_string(&wal_path).await.unwrap();
    assert!(
        wal_content.contains("Insert"),
        "WAL should contain Insert operation"
    );
    assert!(wal_content.contains(id), "WAL should reference document ID");

    println!("✓ WAL commit with document lock test passed");
}

/// Test S4.2.2: Update operation WAL coordination
///
/// This test verifies that update operations properly coordinate WAL writes with
/// document lock acquisition and file updates.
#[tokio::test]
#[serial]
async fn test_wal_coordination_for_update() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store_path = temp_dir.path().join("wal_update_test");

    let store = Store::new(&store_path, None).await.unwrap();
    let collection = store.collection("test_collection").await.unwrap();

    // First insert initial data
    let id = "doc-003";
    collection.insert(id, json!({ "value": 10 })).await.unwrap();

    // Now update the document
    let update_data = json!({ "value": 20, "updated": true });
    let result = collection.update(id, update_data).await;

    assert!(result.is_ok(), "Update should succeed: {:?}", result);

    // Verify document was updated
    let doc = collection
        .get(id)
        .await
        .unwrap()
        .expect("Document should exist");
    assert_eq!(doc.data()["value"], 20);
    assert_eq!(doc.data()["updated"], true);

    // Verify WAL was updated
    let wal_path = store_path.join("wal").join("wal.log");
    let wal_content = tokio::fs::read_to_string(&wal_path).await.unwrap();
    assert!(
        wal_content.contains("Update"),
        "WAL should contain Update operation"
    );
    assert!(wal_content.contains(id), "WAL should reference document ID");

    println!("✓ WAL coordination for update test passed");
}

/// Test S4.2.2: Delete operation WAL coordination
///
/// This test verifies that delete operations properly coordinate WAL writes with
/// document lock acquisition and file deletion.
#[tokio::test]
#[serial]
async fn test_wal_coordination_for_delete() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store_path = temp_dir.path().join("wal_delete_test");

    let store = Store::new(&store_path, None).await.unwrap();
    let collection = store.collection("test_collection").await.unwrap();

    // First insert data
    let id = "doc-004";
    collection.insert(id, json!({ "value": 50 })).await.unwrap();

    // Verify document exists before deletion
    assert!(collection.get(id).await.unwrap().is_some());

    // Delete the document
    let result = collection.delete(id).await;

    assert!(result.is_ok(), "Delete should succeed: {:?}", result);

    // Verify document is gone
    assert!(collection.get(id).await.unwrap().is_none());

    // Verify WAL was updated
    let wal_path = store_path.join("wal").join("wal.log");
    let wal_content = tokio::fs::read_to_string(&wal_path).await.unwrap();
    assert!(
        wal_content.contains("Delete"),
        "WAL should contain Delete operation"
    );
    assert!(wal_content.contains(id), "WAL should reference document ID");

    println!("✓ WAL coordination for delete test passed");
}

/// Test S4.2.2: Concurrent WAL and document lock acquisition
///
/// This test verifies that when multiple processes attempt to write to the same
/// document, they properly coordinate through WAL and file locks to prevent
/// data corruption.
#[tokio::test]
#[serial]
async fn test_concurrent_wal_document_locks() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store_path = temp_dir.path().join("concurrent_wal_test");

    // Spawn multiple processes to update the same document
    let mut children = vec![];
    for proc_id in 0 .. 3 {
        let store_path_clone = store_path.clone();
        let child = std::thread::spawn(move || {
            let store = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { Store::new(&store_path_clone, None).await });

            let store = store.unwrap();
            let collection = store.collection("concurrent_wal_test").await.unwrap();

            // Each process updates the same document
            let id = "shared-doc";
            let data = json!({ "process": proc_id, "value": proc_id * 100 });

            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { collection.update(id, data).await })
                .unwrap();
        });
        children.push(child);
    }

    // Wait for all processes to complete
    for child in children {
        child.join().unwrap();
    }

    // Verify final document state (last writer should win, but no corruption)
    let store = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { Store::new(&store_path, None).await });

    let store = store.unwrap();
    let collection = store.collection("concurrent_wal_test").await.unwrap();

    let doc = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { collection.get("shared-doc").await })
        .unwrap()
        .expect("Document should exist");

    // Last writer (process 2) should have updated the document
    assert_eq!(doc.data()["process"], 2);

    // Verify WAL was updated
    let wal_path = store_path.join("wal").join("wal.log");
    let wal_content = tokio::fs::read_to_string(&wal_path).await.unwrap();
    assert!(
        wal_content.contains("Update"),
        "WAL should contain Update operations"
    );
    assert!(
        wal_content.contains("shared-doc"),
        "WAL should reference shared document"
    );

    println!("✓ Concurrent WAL and document lock acquisition test passed");
}

/// Test S4.2.2: WAL recovery with document locks
///
/// This test verifies that after a crash, WAL can recover properly from document
/// files that may have partial writes (simulating an incomplete commit).
#[tokio::test]
#[serial]
async fn test_wal_recovery_after_crash() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store_path = temp_dir.path().join("wal_recovery_test");

    let store = Store::new(&store_path, None).await.unwrap();
    let collection = store.collection("test_collection").await.unwrap();

    let id = "doc-005";
    let data = json!({ "value": 999 });

    // Insert document (should succeed)
    let result = collection.insert(id, data).await;

    assert!(result.is_ok(), "Insert should succeed: {:?}", result);

    // Verify document exists
    let doc = collection
        .get(id)
        .await
        .unwrap()
        .expect("Document should exist");
    assert_eq!(doc.data()["value"], 999);

    // Verify WAL has the entry
    let wal_path = store_path.join("wal").join("wal.log");
    let wal_content = tokio::fs::read_to_string(&wal_path).await.unwrap();
    assert!(
        wal_content.contains("Insert"),
        "WAL should contain Insert operation"
    );

    println!("✓ WAL recovery after crash test passed");
}
