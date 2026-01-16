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
            if path.is_file()
                && let Some(file_name) = path.file_name().and_then(|n| n.to_str())
                    && file_name.ends_with(".json") && !file_name.starts_with('.') {
                        let id = &file_name[..file_name.len() - 5]; // remove .json
                        yield Ok(id.to_owned());
                    }
        }
    })
}
