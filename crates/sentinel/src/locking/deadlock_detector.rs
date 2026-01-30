//! # Deadlock Detector
//!
//! Implements deadlock detection using a wait-for graph algorithm.
//! Monitors lock requests and detects circular dependencies that cause deadlocks.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::RwLock;

use crate::error::{Result, SentinelError};

/// Deadlock detection state for a single lock request.
#[derive(Debug)]
pub struct LockRequest {
    /// Path being requested
    pub path:         PathBuf,
    /// Strategy requested
    pub strategy:     super::LockStrategy,
    /// When the request started
    pub started_at:   Instant,
    /// Timeout for this request
    pub timeout:      Duration,
    /// ID of the requester (for deadlock detection)
    pub requester_id: String,
}

/// Global deadlock detector that monitors all lock requests.
///
/// This detector maintains a wait-for graph where each node represents a lock requester,
/// and each edge represents a waiting relationship. Cycles in this graph indicate
/// deadlocks, which are detected and resolved by aborting the youngest transaction.
#[derive(Debug)]
pub struct DeadlockDetector {
    /// Wait-for graph: requester_id -> set of holders that requester is waiting for
    wait_graph:       RwLock<HashMap<String, Vec<String>>>,
    /// Current lock requests: path -> list of pending requests
    pending_requests: RwLock<HashMap<PathBuf, Vec<LockRequest>>>,
    /// Active locks: path -> (holder_id, strategy)
    active_locks:     RwLock<HashMap<PathBuf, (String, super::LockStrategy)>>,
}

impl DeadlockDetector {
    /// Create a new deadlock detector.
    pub fn new() -> Self {
        Self {
            wait_graph:       RwLock::new(HashMap::new()),
            pending_requests: RwLock::new(HashMap::new()),
            active_locks:     RwLock::new(HashMap::new()),
        }
    }

    /// Register that a requester is waiting for a lock held by holder_id.
    pub async fn register_wait(&self, requester_id: String, holder_id: String) -> Result<()> {
        let mut graph = self.wait_graph.write().await;
        graph
            .entry(requester_id)
            .or_insert_with(Vec::new)
            .push(holder_id);
        Ok(())
    }

    /// Remove a wait relationship when a request completes.
    pub async fn unregister_wait(&self, requester_id: String, holder_id: String) -> Result<()> {
        let mut graph = self.wait_graph.write().await;
        if let Some(holders) = graph.get_mut(&requester_id) {
            holders.retain(|id| id != &holder_id);
            if holders.is_empty() {
                graph.remove(&requester_id);
            }
        }
        Ok(())
    }

    /// Record an active lock acquisition.
    pub async fn record_lock_acquired(&self, path: &Path, holder_id: String, strategy: super::LockStrategy) {
        let mut active = self.active_locks.write().await;
        active.insert(path.to_path_buf(), (holder_id, strategy));
    }

    /// Record a lock release.
    pub async fn record_lock_released(&self, path: &Path) {
        let mut active = self.active_locks.write().await;
        active.remove(path);
    }

    /// Register a pending lock request.
    pub async fn register_request(&self, request: LockRequest) {
        let mut pending = self.pending_requests.write().await;
        pending
            .entry(request.path.clone())
            .or_insert_with(Vec::new)
            .push(request);
    }

    /// Remove a pending request when it completes.
    pub async fn unregister_request(&self, path: &Path, requester_id: String) {
        let mut pending = self.pending_requests.write().await;
        if let Some(requests) = pending.get_mut(path) {
            requests.retain(|req| req.requester_id != requester_id);
            if requests.is_empty() {
                pending.remove(path);
            }
        }
    }

    /// Detect if there's a deadlock involving the given requester.
    /// Returns true if a cycle is found in the wait-for graph.
    pub async fn detect_deadlock(&self, requester_id: String) -> bool {
        let graph = self.wait_graph.read().await;
        let mut visited = std::collections::HashSet::new();
        let mut path: Vec<String> = Vec::new();
        let mut current_path = std::collections::HashSet::new();

        Self::has_cycle(
            &graph,
            &requester_id,
            &mut path,
            &mut current_path,
            &mut visited,
        )
    }

    /// Check for deadlocks and return IDs of deadlocked transactions.
    pub async fn find_deadlocked_transactions(&self) -> Vec<String> {
        let graph = self.wait_graph.read().await;
        let mut deadlocked = Vec::new();
        let mut visited = std::collections::HashSet::new();

        for start_node in graph.keys() {
            if visited.contains(start_node) {
                continue;
            }

            let mut path: Vec<String> = Vec::new();
            let mut current_path = std::collections::HashSet::new();

            if Self::has_cycle(
                &graph,
                &start_node,
                &mut path,
                &mut current_path,
                &mut visited,
            ) {
                // Find the transaction to abort (youngest in the cycle)
                if let Some(victim) = path.last() {
                    deadlocked.push(victim.clone());
                }
            }
        }

        deadlocked
    }

    /// Helper function to detect cycles in the wait-for graph using DFS.
    fn has_cycle(
        graph: &HashMap<String, Vec<String>>,
        node: &String,
        path: &mut Vec<String>,
        current_path: &mut std::collections::HashSet<String>,
        visited: &mut std::collections::HashSet<String>,
    ) -> bool {
        // If this node is in the current traversal path, we found a cycle
        if current_path.contains(node) {
            return true;
        }

        // Mark this node as visited
        if visited.contains(node) {
            return false;
        }
        visited.insert(node.clone());
        current_path.insert(node.clone());
        path.push(node.clone());

        // Recursively check neighbors
        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) && Self::has_cycle(graph, neighbor, path, current_path, visited) {
                    return true;
                }
                else if current_path.contains(neighbor) {
                    return true;
                }
            }
        }

        // Backtrack: remove node from current path
        current_path.remove(node);
        path.pop();
        false
    }
}
