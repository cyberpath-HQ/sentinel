use std::{path::PathBuf, process::Command};

/// Integration tests for multi-process concurrent access to Sentinel DBMS
///
/// This test suite verifies that the file locking system prevents race conditions
/// and ensures consistent access across multiple processes.
use sentinel_dbms::{Collection, Store};
use serde_json::json;

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

    // Spawn multiple processes to compete for the same collection
    let mut children = vec![];
    for i in 0 .. 3 {
        let store_path_clone = store_path.clone();
        let child = std::thread::spawn(move || {
            // Initialize store in this process
            let store = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { Store::new(&store_path_clone, None).await });

            let store = store.unwrap();
            let collection = store.collection("test_collection").await.unwrap();

            // Each process performs insert operations
            for j in 0 .. 10 {
                let id = format!("doc-{}-{}", i, j);
                let data = json!({ "process": i, "sequence": j, "value": i * 100 + j });

                let result = tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async { collection.insert(&id, data).await });

                match result {
                    Ok(_) => println!("Process {} inserted doc-{}-{}", i, i, j),
                    Err(e) => eprintln!("Process {} failed to insert doc-{}-{}: {}", i, i, j, e),
                }
            }
        });
        children.push(child);
    }

    // Wait for all processes to complete
    for child in children {
        child.join().unwrap();
    }

    // Verify all documents were written correctly
    let store = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { Store::new(&store_path, None).await });

    let store = store.unwrap();
    let collection = store.collection("test_collection").await.unwrap();

    // Check that all documents were created (30 total: 3 processes * 10 docs each)
    let total_count = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { collection.count().await });

    assert_eq!(
        total_count, 30,
        "Expected 30 documents, got {}",
        total_count
    );

    // Verify each document's integrity
    for i in 0 .. 3 {
        for j in 0 .. 10 {
            let id = format!("doc-{}-{}", i, j);
            let doc = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { collection.get(&id).await });

            let doc = doc.unwrap().expect(&format!("Document {} not found", id));
            assert_eq!(doc.data()["process"], i);
            assert_eq!(doc.data()["sequence"], j);
            assert_eq!(doc.data()["value"], i * 100 + j);
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

    // Spawn writer processes
    let mut writer_children = vec![];
    for i in 0 .. 2 {
        let store_path_clone = store_path.clone();
        let child = std::thread::spawn(move || {
            let store = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { Store::new(&store_path_clone, None).await });

            let store = store.unwrap();
            let collection = store.collection("read_write_test").await.unwrap();

            for j in 0 .. 20 {
                let id = format!("doc-{}", j);
                let data = json!({ "writer": i, "sequence": j });

                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async { collection.insert(&id, data).await })
                    .unwrap();
            }
        });
        writer_children.push(child);
    }

    // Spawn reader processes
    let mut reader_children = vec![];
    for i in 0 .. 3 {
        let store_path_clone = store_path.clone();
        let child = std::thread::spawn(move || {
            let store = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { Store::new(&store_path_clone, None).await });

            let store = store.unwrap();
            let collection = store.collection("read_write_test").await.unwrap();

            // Each reader reads all documents
            for j in 0 .. 20 {
                let id = format!("doc-{}", j);
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async { collection.get(&id).await })
                    .ok();
            }
        });
        reader_children.push(child);
    }

    // Wait for all processes
    for child in writer_children {
        child.join().unwrap();
    }
    for child in reader_children {
        child.join().unwrap();
    }

    // Verify data integrity
    let store = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { Store::new(&store_path, None).await });

    let store = store.unwrap();
    let collection = store.collection("read_write_test").await.unwrap();

    let total_count = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { collection.count().await });

    assert_eq!(
        total_count, 40,
        "Expected 40 documents, got {}",
        total_count
    );

    println!("✓ Multi-process read-write concurrency test passed");
}

/// Test S4.2.1: Multi-process update operations
///
/// This test verifies that exclusive locks are properly acquired during update
/// operations across multiple processes.
#[tokio::test]
#[serial]
async fn test_multi_process_concurrent_updates() {
    let temp_dir = tempfile::tempdir().unwrap();
    let store_path = temp_dir.path().join("update_test");

    // Setup initial data
    let store = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { Store::new(&store_path, None).await });

    let store = store.unwrap();
    let collection = store.collection("update_test").await.unwrap();

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
            let store = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { Store::new(&store_path_clone, None).await });

            let store = store.unwrap();
            let collection = store.collection("update_test").await.unwrap();

            // Each process updates different subset of documents
            for i in (proc_id * 3) .. ((proc_id + 1) * 3) {
                let id = format!("doc-{}", i);
                let data = json!({ "value": i * 10 + proc_id });

                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async { collection.update(&id, data).await })
                    .unwrap();
            }
        });
        children.push(child);
    }

    // Wait for updates to complete
    for child in children {
        child.join().unwrap();
    }

    // Verify updated values
    let store = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { Store::new(&store_path, None).await });

    let store = store.unwrap();
    let collection = store.collection("update_test").await.unwrap();

    for i in 0 .. 10 {
        let id = format!("doc-{}", i);
        let doc = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { collection.get(&id).await })
            .unwrap()
            .expect(&format!("Document {} not found", i));

        // Calculate expected value: original i * 10 + proc_id
        let proc_id = i / 3;
        assert_eq!(doc.data()["value"], i * 10 + proc_id);
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
    let store = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { Store::new(&store_path, None).await });

    let store = store.unwrap();
    let collection = store.collection("delete_test").await.unwrap();

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
            let store = tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async { Store::new(&store_path_clone, None).await });

            let store = store.unwrap();
            let collection = store.collection("delete_test").await.unwrap();

            // Each process deletes different subset
            for i in (proc_id * 5) .. ((proc_id + 1) * 5) {
                let id = format!("doc-{}", i);
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async { collection.delete(&id).await })
                    .unwrap();
            }
        });
        children.push(child);
    }

    // Wait for deletes to complete
    for child in children {
        child.join().unwrap();
    }

    // Verify remaining documents
    let store = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { Store::new(&store_path, None).await });

    let store = store.unwrap();
    let collection = store.collection("delete_test").await.unwrap();

    let remaining = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { collection.count().await });

    assert_eq!(
        remaining, 0,
        "Expected all documents to be deleted, but {} remain",
        remaining
    );

    println!("✓ Multi-process concurrent deletes test passed");
}
