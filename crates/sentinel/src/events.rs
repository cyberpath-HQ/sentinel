/// Event system for synchronizing store metadata with collection operations.
///
/// This module defines the event types that collections emit when operations occur,
/// allowing the store to maintain accurate metadata without requiring wrapper methods.
use serde::{Deserialize, Serialize};

/// Events emitted by collections to notify the store of metadata changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
