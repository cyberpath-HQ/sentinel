/// Event system for synchronizing store metadata with collection operations.
///
/// This module defines the event types that collections emit when operations occur,
/// allowing the store to maintain accurate metadata without requiring wrapper methods.
use serde::{Deserialize, Serialize};

/// Events emitted by collections to notify the store of metadata changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StoreEvent {
    /// A new collection was created.
    CollectionCreated {
        /// Name of the collection that was created.
        name: String,
    },
    /// A collection was deleted.
    CollectionDeleted {
        /// Name of the collection that was deleted.
        name:             String,
        /// Number of documents that were in the collection.
        document_count:   u64,
        /// Total size in bytes of all documents in the collection.
        total_size_bytes: u64,
    },
    /// A document was inserted into a collection.
    DocumentInserted {
        /// Name of the collection.
        collection: String,
        /// Size in bytes of the inserted document.
        size_bytes: u64,
    },
    /// A document was updated in a collection.
    DocumentUpdated {
        /// Name of the collection.
        collection:     String,
        /// Size in bytes of the document before the update.
        old_size_bytes: u64,
        /// Size in bytes of the document after the update.
        new_size_bytes: u64,
    },
    /// A document was deleted from a collection.
    DocumentDeleted {
        /// Name of the collection.
        collection: String,
        /// Size in bytes of the deleted document.
        size_bytes: u64,
    },
}

/// Trait for types that can emit store events.
pub trait EventEmitter {
    /// Emit an event to the store.
    fn emit_event(&self, event: StoreEvent);
}

impl EventEmitter for crate::Collection {
    fn emit_event(&self, event: StoreEvent) {
        if let Some(sender) = &self.event_sender {
            let _ = sender.send(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_event_serialization() {
        let events = vec![
            StoreEvent::CollectionCreated {
                name: "test_collection".to_string(),
            },
            StoreEvent::CollectionDeleted {
                name:             "test_collection".to_string(),
                document_count:   42,
                total_size_bytes: 1024,
            },
            StoreEvent::DocumentInserted {
                collection: "test_collection".to_string(),
                size_bytes: 256,
            },
            StoreEvent::DocumentUpdated {
                collection:     "test_collection".to_string(),
                old_size_bytes: 128,
                new_size_bytes: 256,
            },
            StoreEvent::DocumentDeleted {
                collection: "test_collection".to_string(),
                size_bytes: 256,
            },
        ];

        for event in events {
            let serialized = serde_json::to_string(&event).unwrap();
            let deserialized: StoreEvent = serde_json::from_str(&serialized).unwrap();
            assert_eq!(event, deserialized);
        }
    }

    #[test]
    fn test_store_event_debug() {
        let event = StoreEvent::DocumentInserted {
            collection: "users".to_string(),
            size_bytes: 512,
        };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("DocumentInserted"));
        assert!(debug_str.contains("users"));
        assert!(debug_str.contains("512"));
    }
}
