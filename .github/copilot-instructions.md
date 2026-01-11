# Cyberpath Sentinel AI Coding Guidelines

> **IMPORTANT NOTE**: Never leave TODOs, stub code, or uncompleted functions unless explicitly asked to do so. All code
> must be fully implemented, tested, and documented.

## Project Overview

Cyberpath Sentinel is a filesystem-backed document DBMS written in Rust. It stores data as JSON files in directories
representing collections, with filenames as document IDs. The system prioritizes auditability, compliance, and
transparency over raw performance, making it ideal for audit logs, certificate management, and regulatory data.

## Architecture

- **Store**: Top-level manager for collections, handles global config, encryption, and caching (see
  `IMPLEMENTATION_PLAN.md` for Store struct)
- **Collection**: Manages documents within a directory, includes index manager, transaction log, and lock manager
- **Document**: JSON file with embedded metadata (version, timestamps, hash, signature)
- Directory structure: `data/collection/.metadata.json` (indices, settings), `.deleted/` (soft-deleted docs), `.index/`
  (lazy indices), `.wal/` (write-ahead log)

## Key Patterns

- **Async I/O**: All file operations use async/await with tokio; avoid blocking calls
- **Serialization**: Use `serde_json::Value` for documents; binary WAL entries with bincode
- **Error Handling**: Return `io::Result<T>`; use `?` for propagation
- **Locking**: Exclusive locks for writes (fs2 crate), shared for reads
- **Encryption**: AES-256-GCM for data, RSA-2048 for signatures; encrypt specific fields only
- **Hashing**: SHA-256 for document integrity, CRC32 for WAL entries

## Development Workflow

- Build: `cargo build` (workspace setup with crates/sentinel/)
- Test: `cargo test` (run from root)
- Format: `cargo fmt --all`
- Lint: `cargo clippy --all-targets --all-features`
- Debug: Use `println!` for logging; no custom logger yet

## Conventions

- Document IDs: String-based, filename-safe (e.g., `user-123.json`)
- Metadata: `_id`, `_version`, `_created_at`, `_updated_at`, `_hash`, `_signature`
- Soft Deletes: Move files to `.deleted/` subdirectory
- Indices: Hash-based for exact matches, stored as serialized DashMap in `.index/`
- Transactions: Log to WAL before committing file changes

## Quality Assurance

- **Documentation**: For each feature or function implemented, provide thorough documentation at the function level (doc
  comments) and inline comments explaining complex logic. All public APIs must have comprehensive docs explaining
  purpose, parameters, return values, and examples.
- **Unit Testing**: Each and every function must be unit tested wherever it is located. Tests must cover edge cases
  (e.g., empty inputs, invalid data, boundary conditions). Test coverage must be at least 90% across the codebase. Tests
  should be documented as standard code with clear names and comments.
- **Benchmarking**: For each non-test function, wherever it is, define benchmarks using `criterion` crate to measure
  performance. Benchmarks must check both best-case and worst-case path executions each and every time, ensuring
  performance regressions are caught early.

## Examples

- Insert document: `users.insert("user-123", json!({ "name": "Alice" })).await?`
- Get document: Read `data/users/user-123.json`, deserialize to Document struct
- Query: Walk directory, filter documents in-memory using predicates
- WAL entry: Binary format with entry type, transaction ID, collection, doc ID, data, CRC

Reference `IMPLEMENTATION_PLAN.md` for component APIs and phase details.
