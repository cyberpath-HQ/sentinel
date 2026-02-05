//! # Lock Strategy
//!
//! Defines the locking strategies available for file operations.
//! Exclusive locks allow single writer access, shared locks allow multiple readers.

/// Lock strategy determines how locks are acquired
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockStrategy {
    /// Exclusive lock: Single writer, blocks all other access
    Exclusive,
    /// Shared lock: Multiple readers, blocks writers
    Shared,
}

impl LockStrategy {
    /// Check if this strategy is compatible with an existing lock
    pub fn is_compatible(&self, existing: Self) -> bool {
        match (self, existing) {
            // Shared with shared is compatible
            (Self::Shared, Self::Shared) => true,
            // Exclusive with anything is incompatible
            (Self::Exclusive, _) => false,
            // Shared with exclusive is incompatible
            (Self::Shared, Self::Exclusive) => false,
        }
    }
}
