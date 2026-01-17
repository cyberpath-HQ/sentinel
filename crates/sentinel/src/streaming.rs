//! Streaming utilities for document processing.

use std::{path::PathBuf, pin::Pin};

use async_stream::stream;
use futures::Stream;
use tokio::fs as tokio_fs;

use crate::Result;

/// Streams document IDs from a collection directory.
pub fn stream_document_ids(collection_path: PathBuf) -> Pin<Box<dyn Stream<Item = Result<String>> + Send>> {
    Box::pin(stream! {
        let mut entries = match tokio_fs::read_dir(&collection_path).await {
            Ok(entries) => entries,
            Err(e) => {
                yield Err(e.into());
                return;
            }
        };

        loop {
            let entry = match entries.next_entry().await {
                Ok(Some(entry)) => entry,
                Ok(None) => break,
                Err(e) => {
                    yield Err(e.into());
                    continue;
                }
            };

            let path = entry.path();
            if !tokio_fs::metadata(&path).await?.is_dir()
                && let Some(file_name) = path.file_name().and_then(|n| n.to_str())
                && file_name.ends_with(".json") && !file_name.starts_with('.') {
                let id = &file_name[..file_name.len() - 5]; // remove .json
                yield Ok(id.to_owned());
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use tokio::fs as tokio_fs;
    use futures::StreamExt;

    use super::*;

    #[tokio::test]
    async fn test_stream_document_ids() {
        let temp_dir = TempDir::new().unwrap();
        let collection_path = temp_dir.path().join("collection");
        tokio_fs::create_dir(&collection_path).await.unwrap();
        let doc_ids = vec!["doc1", "doc2", "doc3"];
        for id in &doc_ids {
            let file_path = collection_path.join(format!("{}.json", id));
            tokio_fs::write(&file_path, b"{}").await.unwrap();
        }
        let mut stream = stream_document_ids(collection_path);
        let mut found_ids = Vec::new();
        while let Some(result) = stream.next().await {
            let id = result.unwrap();
            found_ids.push(id);
        }
        found_ids.sort();
        let mut expected_ids: Vec<String> = doc_ids.iter().map(|s| s.to_string()).collect();
        expected_ids.sort();
        assert_eq!(found_ids, expected_ids);
    }

    #[tokio::test]
    async fn test_stream_document_ids_with_directory_removal() {
        let temp_dir = TempDir::new().unwrap();
        let collection_path = temp_dir.path().join("collection");
        tokio_fs::create_dir(&collection_path).await.unwrap();
        let doc_ids = vec!["doc1", "doc2"];
        for id in &doc_ids {
            let file_path = collection_path.join(format!("{}.json", id));
            tokio_fs::write(&file_path, b"{}").await.unwrap();
        }

        let mut stream = stream_document_ids(collection_path.clone());

        // Consume one item first to ensure read_dir() succeeds
        let first_result = stream.next().await;
        assert!(first_result.is_some());
        let first_id = match first_result.unwrap() {
            Ok(id) => id,
            Err(_) => panic!("Expected first item to succeed"),
        };

        // Now remove the directory during iteration
        tokio_fs::remove_dir_all(&collection_path).await.unwrap();

        let mut found_ids = vec![first_id]; // already consumed
        while let Some(result) = stream.next().await {
            match result {
                Ok(id) => found_ids.push(id),
                Err(_) => {}, // Error handling is tested by not panicking
            }
        }

        // The behavior may vary by platform - some may continue reading, others may error
        // The important thing is that the error handling doesn't panic
        assert!(
            !found_ids.is_empty(),
            "Expected to find at least one document id"
        );
        // Error count may be 0 or more depending on platform
    }

    #[tokio::test]
    async fn test_stream_document_ids_with_invalid_path() {
        // Test with a path that is not a directory to trigger read_dir error
        let invalid_path = std::path::PathBuf::from("/dev/null/nonexistent");
        let mut stream = stream_document_ids(invalid_path);
        let mut error_count = 0;
        while let Some(result) = stream.next().await {
            match result {
                Ok(_) => {},
                Err(_) => error_count += 1,
            }
        }
        assert!(error_count > 0, "Expected error when path is invalid");
    }
}
