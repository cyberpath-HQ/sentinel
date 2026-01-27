use tracing::{debug, error, trace, warn};

use crate::{events::StoreEvent, StoreMetadata, META_SENTINEL_VERSION, STORE_METADATA_FILE};
use super::stor::Store;

/// Starts the background event processing task.
#[allow(clippy::integer_division_remainder_used, reason = "integer division used in event processing timing")]
pub fn start_event_processor(store: &mut Store) {
    if store.event_task.is_some() {
        return; // Already started
    }

    // Take ownership of the event receiver
    let Some(mut receiver) = store.event_receiver.take()
    else {
        warn!("Event receiver already taken");
        return;
    };

    // Clone the counters and config for the background task
    let total_size_bytes = store.total_size_bytes.clone();
    let total_documents = store.total_documents.clone();
    let collection_count = store.collection_count.clone();
    let stored_wal_config = store.stored_wal_config.clone();
    let root_path = store.root_path.clone();
    let created_at = store.created_at;

    let task = tokio::spawn(async move {
        // Debouncing: save metadata every 500 milliseconds instead of after every event
        let mut save_interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
        save_interval.tick().await; // First tick completes immediately

        let mut changed = false;

        loop {
            tokio::select! {
                // Process events
                event = receiver.recv() => {
                    match event {
                        Some(StoreEvent::CollectionCreated { name }) => {
                            debug!("Processing collection created event: {}", name);
                            collection_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            changed = true;
                        }
                        Some(StoreEvent::CollectionDeleted { name, document_count, total_size_bytes: event_size }) => {
                            debug!("Processing collection deleted event: {} (docs: {}, size: {})",
                                  name, document_count, event_size);
                            collection_count.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                            total_documents.fetch_sub(document_count, std::sync::atomic::Ordering::Relaxed);
                            total_size_bytes.fetch_sub(event_size, std::sync::atomic::Ordering::Relaxed);
                            changed = true;
                        }
                        Some(StoreEvent::DocumentInserted { collection, size_bytes }) => {
                            debug!("Processing document inserted event: {} (size: {})", collection, size_bytes);
                            total_documents.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            total_size_bytes.fetch_add(size_bytes, std::sync::atomic::Ordering::Relaxed);
                            changed = true;
                        }
                        Some(StoreEvent::DocumentUpdated { collection, old_size_bytes, new_size_bytes }) => {
                            debug!("Processing document updated event: {} (old: {}, new: {})",
                                  collection, old_size_bytes, new_size_bytes);
                            total_size_bytes.fetch_sub(old_size_bytes, std::sync::atomic::Ordering::Relaxed);
                            total_size_bytes.fetch_add(new_size_bytes, std::sync::atomic::Ordering::Relaxed);
                            changed = true;
                        }
                        Some(StoreEvent::DocumentDeleted { collection, size_bytes }) => {
                            debug!("Processing document deleted event: {} (size: {})", collection, size_bytes);
                            total_documents.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                            total_size_bytes.fetch_sub(size_bytes, std::sync::atomic::Ordering::Relaxed);
                            changed = true;
                        }
                        None => {
                            // Channel closed, exit
                            break;
                        }
                    }
                }

                // Periodic metadata save
                _ = save_interval.tick() => {
                    if changed {
                        let metadata = StoreMetadata {
                            version: META_SENTINEL_VERSION,
                            created_at: created_at.timestamp() as u64,
                            updated_at: chrono::Utc::now().timestamp() as u64,
                            collection_count: collection_count.load(std::sync::atomic::Ordering::Relaxed),
                            total_documents: total_documents.load(std::sync::atomic::Ordering::Relaxed),
                            total_size_bytes: total_size_bytes.load(std::sync::atomic::Ordering::Relaxed),
                            wal_config: stored_wal_config.clone(),
                        };

                        let content = match serde_json::to_string_pretty(&metadata) {
                            Ok(content) => content,
                            Err(e) => {
                                error!("Failed to serialize store metadata: {}", e);
                                continue;
                            }
                        };

                        let metadata_path = root_path.join(STORE_METADATA_FILE);
                        if let Err(e) = tokio::fs::write(&metadata_path, content).await {
                            error!("Failed to save store metadata in background task: {}", e);
                        } else {
                            trace!("Store metadata saved successfully");
                            changed = false;
                        }
                    }
                }
            }
        }
    });

    store.event_task = Some(task);
}
