<div align="center">
  <picture>
    <source srcset="https://raw.githubusercontent.com/cyberpath-HQ/sentinel/refs/heads/main/.assets/logo-white.svg" media="(prefers-color-scheme: dark)" />
    <source srcset="https://raw.githubusercontent.com/cyberpath-HQ/sentinel/refs/heads/main/.assets/logo.svg" media="(prefers-color-scheme: light)" />
    <img src="https://raw.githubusercontent.com/cyberpath-HQ/sentinel/refs/heads/main/.assets/logo.svg" alt="Sentinel Logo" height="64"/>
  </picture>

[![Cyberpath](https://img.shields.io/badge/Cyberpath-project-blue)](https://sentinel.cyberpath-hq.com)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-green.svg)](LICENSE.md)
![Codecov](https://img.shields.io/codecov/c/github/cyberpath-HQ/sentinel)
[![Crates.io Version](https://img.shields.io/crates/v/sentinel-dbms)](https://crates.io/crates/sentinel-dbms)
[![docs.rs](https://img.shields.io/docsrs/sentinel-dbms)](https://docs.rs/sentinel-dbms/latest/sentinel_dbms/)

</div>

A document-based DBMS written in Rust that stores all data as files on disk, where tables are represented by folders and
each document's primary key is the filename. Every piece of data is inspectable, auditable, and compliant by design.

---

## Why Cyberpath Sentinel?

Modern databases prioritize speed. Cyberpath Sentinel prioritizes **trust, transparency, and compliance**.

### Perfect For

- **Audit Logs** - Every entry is a file, versioned with Git
- **Certificate Management** - Secure, inspectable, with OS-level ACLs
- **Compliance Rules & Policies** - GDPR right-to-delete is literally `rm file`
- **Encryption Key Management** - Keys stored as files with filesystem security
- **Regulatory Reporting** - All data is immediately forensic-friendly
- **Edge Devices & Disconnected Systems** - No server required, works with Git sync
- **Zero-Trust Infrastructure** - Inspect everything before trusting it

### NOT For

- Real-time bidding systems
- High-frequency trading platforms
- Streaming analytics pipelines
- Multi-million row transactional systems (yet)

---

## Massive Advantages

### **Auditability & Security**

- Every document is inspectable with `cat` or your favorite editor
- Versioned transparently with Git—see who changed what, when, and why
- Secured with OS-level ACLs; no database user management nonsense
- Cryptographic hashing enables forensic integrity verification

### **Operational Simplicity**

- Use standard UNIX tools: `rsync` for replication, `tar` for backups, `grep` for queries
- No database daemon to manage, update, or patch
- Deploy to any device with a filesystem; scaling is adding folders
- Disaster recovery: `git clone` and you're done

### **Compliance-Ready**

- GDPR right-to-delete: `rm file.json` and it's gone (with audit trail)
- Immutable audit logs with append-only patterns
- PII can be encrypted at rest and in transit with cryptographic keys
- Regulatory bodies love "show me the data"—here it is, plain text

### **Zero Lock-In**

- Data is pure JSON/BSON—no proprietary binary formats
- Migrate to PostgreSQL, MongoDB, or DuckDB using standard tools
- Your data isn't trapped in a vendor ecosystem
- Export to CSV, XML, or custom formats trivially

### **Perfect Secure Ecosystem**

- Integrates seamlessly with security tools and compliance frameworks
- Designed for organizations managing sensitive security and compliance data
- Audit trails that satisfy SOC 2, ISO 27001, HIPAA requirements
- Natural fit for DevOps, SRE, and security operations teams

---

## Real Trade-offs (We're Honest)

### **Concurrency Complexity**

- Multi-writer scenarios require file locks or Write-Ahead Logging (WAL)
- Not optimized for thousands of concurrent writes
- Proposed Solution: Locking strategies, eventual consistency, and deterministic replication

### **Query Performance**

- No native B-tree indices; initial queries scan files
- Proposed Solution: In-memory caching, lazy indexing, and hash-based sharding

### **Scaling Limits**

- Single folder performance degrades around 4M files
- Proposed Solution: Hash-based sharding, distributed stores, and hierarchical folders

### **Partial Write Safety**

- Power failure mid-write requires careful handling
- Proposed Solution: Write-Ahead Logging, atomic rename patterns, and checksums

### **Not for High-Throughput**

- Bad for 100K+ operations per second
- Good for audit logs, configuration management, and compliance data

---

## ⚡ Quick Start

### Installation

```bash
# Install from crates.io
cargo install sentinel-dbms

# Or add to your Cargo.toml
[dependencies]
sentinel-dbms = "0.1"

# Or build from source
git clone https://github.com/cyberpath-HQ/sentinel
cd sentinel
cargo build --release
```

### Library Usage

```rust
use sentinel_dbms::{Store, QueryBuilder, Operator, SortOrder};
use serde_json::json;
use futures::TryStreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a store (async I/O with optional passphrase for signing)
    let store = Store::new("./data", None).await?;

    // Access a collection (creates data/users/ folder if needed)
    let users = store.collection("users").await?;

    // Insert a document (creates data/users/user-123.json)
    users.insert("user-123", json!({
        "name": "Alice Johnson",
        "email": "alice@example.com",
        "role": "admin",
        "department": "Engineering",
        "age": 30
    })).await?;

    // Retrieve a document
    if let Some(user) = users.get("user-123").await? {
        println!("User: {}", serde_json::to_string_pretty(&user)?);
    }

    // Update a document
    users.update("user-123", json!({
        "name": "Alice Smith",
        "email": "alice.smith@example.com",
        "role": "admin",
        "department": "Engineering",
        "age": 31
    })).await?;

    // Stream all documents (memory-efficient for large collections)
    let stream = users.all();
    let docs: Vec<_> = stream.try_collect().await?;
    println!("Total documents: {}", docs.len());

    // Query with filters using predicate functions
    let adults = users.filter(|doc| {
        doc.data().get("age")
            .and_then(|v| v.as_i64())
            .map_or(false, |age| age >= 18)
    });
    let adult_docs: Vec<_> = adults.try_collect().await?;
    println!("Adults: {}", adult_docs.len());

    // Advanced querying with QueryBuilder
    let query = QueryBuilder::new()
        .filter("age", Operator::GreaterThan, json!(25))
        .filter("role", Operator::Equals, json!("admin"))
        .sort("name", SortOrder::Ascending)
        .limit(10)
        .offset(0)
        .project(vec!["name".to_string(), "email".to_string()])
        .build();

    let result = users.query(query).await?;
    println!("Query executed in {:?}", result.execution_time);

    // Stream query results
    let stream = result.documents;
    futures::pin_mut!(stream);
    while let Some(doc) = stream.try_next().await? {
        println!("Found: {:?}", doc.data());
    }

    // Delete a document (soft delete to .deleted/ folder)
    users.delete("user-123").await?;

    Ok(())
}
```

### CLI Usage

The Sentinel CLI provides commands for managing stores and documents from the terminal:

```bash
# Initialize a store
sentinel init --path ./my-store

# Create a collection
sentinel create-collection --store ./my-store --name users

# Insert a document
sentinel insert \
  --store ./my-store \
  --collection users \
  --id user-1 \
  --data '{"name": "Alice", "email": "alice@example.com"}'

# Query documents
sentinel query \
  --store ./my-store \
  --collection users \
  --filter "age>25" \
  --filter "role=admin" \
  --sort "name:asc" \
  --limit 10 \
  --project "name,email"

# Get a specific document
sentinel get --store ./my-store --collection users --id user-1

# List all documents in a collection
sentinel list --store ./my-store --collection users

# Update a document
sentinel update \
  --store ./my-store \
  --collection users \
  --id user-1 \
  --data '{"name": "Alice Smith", "email": "alice.smith@example.com"}'

# Delete a document
sentinel delete --store ./my-store --collection users --id user-1
```

### Folder Structure

```text
data/
├── users/
│   ├── user-123.json
│   ├── user-456.json
│   └── .deleted/
│       └── user-789.json
├── audit_logs/
│   ├── audit-2026-01-01.json
│   └── audit-2026-01-02.json
└── certs/
    ├── cert-a1b2c3.pem
    └── cert-d4e5f6.pem
```

---

## 📋 Features & Status

### ✅ Implemented Features

- **Document Storage** - JSON files stored as inspectable documents
- **Collections** - Folder-based namespaces for organizing documents
- **Async CRUD Operations** - Full Create, Read, Update, Delete with Tokio
- **Document Metadata** - Automatic version, timestamps, hash, and signature
- **Streaming API** - Memory-efficient streaming for large datasets
- **Advanced Querying** - Filter, sort, limit, offset, and projection
- **Query Builder** - Fluent API for building complex queries
- **Cryptography Module** - Modular hashing, signing, encryption, key derivation
- **Multiple Algorithms**:
  - Hashing: BLAKE3
  - Signing: Ed25519
  - Encryption: XChaCha20-Poly1305, AES-256-GCM-SIV, Ascon-128
  - Key Derivation: Argon2id, PBKDF2
- **Soft Deletes** - Documents moved to `.deleted/` folder
- **CLI Tool** - Complete command-line interface with all operations
- **Passphrase Protection** - Encrypt signing keys with passphrases
- **Global Crypto Config** - Flexible configuration for algorithm selection
- **Comprehensive Testing** - Extensive unit and integration tests
- **Benchmarking** - Performance benchmarks with Criterion
- **WAL (Write-Ahead Logging)** - Durable transaction logging for crash recovery

### 🚧 In Progress

- [ ] File locking for concurrent writes
- [ ] Lazy indexing for improved query performance
- [ ] In-memory caching with LRU eviction

### 📋 Planned Features

- [ ] Full-text search capabilities
- [ ] Replication and sync (Git integration)
- [ ] Backup and restore utilities
- [ ] Compliance reporting dashboards
- [ ] Multi-version concurrency control (MVCC)
- [ ] Access control lists (ACLs)
- [ ] Content-addressable storage
- [ ] Merkle tree verification for integrity

---

## Architecture

### Design Principles

1. **Filesystem is the Database** - Leverage OS reliability and tooling
2. **Immutability by Default** - Audit trails, append-only logs, deletions are soft
3. **Security First** - File permissions, encryption, cryptographic verification
4. **UNIX Philosophy** - Do one thing well, compose with standard tools
5. **Zero External Dependencies** - Works offline, on edge devices

### Core Components

```text
┌─────────────────────────────────────────┐
│  Cyberpath Sentinel Client (Rust)       │
├─────────────────────────────────────────┤
│  Query Engine & Filtering               │
│  Transaction Manager (WAL)              │
│  Caching Layer (in-memory)              │
├─────────────────────────────────────────┤
│  File I/O & Concurrency Control         │
│  Encryption & Signing                   │
│  Checksum Verification                  │
├─────────────────────────────────────────┤
│  Filesystem (ext4, NTFS, APFS, etc.)    │
│  OS-level ACLs & Permissions            │
└─────────────────────────────────────────┘
```

---

## Security & Compliance

### Built-In Security

- **Filesystem Permissions** - Leverage OS ACLs for access control
- **Encryption at Rest** - Optional AES-256 encryption for sensitive files
- **Checksums & Integrity** - SHA-256 hashing for corruption detection
- **Immutable Audit Logs** - Append-only journals with cryptographic signatures
- **Soft Deletes** - Deleted files moved to `.deleted/` folder (recoverable)

### Compliance Ready

- **GDPR** - Right-to-delete is filesystem deletion
- **SOC 2** - Audit trails are intrinsic to the system
- **HIPAA** - Encryption at rest and in transit
- **PCI-DSS** - File-level access controls
- **ISO 27001** - Security controls built into architecture

---

## Performance Characteristics

### Best Case Scenarios

| Operation | Time Complexity | Notes                             |
| --------- | --------------- | --------------------------------- |
| Insert    | O(1)            | Single file write                 |
| Get       | O(1)            | Direct filename lookup            |
| Delete    | O(1)            | Rename to .deleted/               |
| Update    | O(1)            | Atomic file rename                |
| List      | O(n)            | Scan directory for filenames      |
| Filter    | O(n)            | Scan all files in collection      |
| Index     | O(n)            | Build lazy indices on first query |

### Optimization Strategies

- **Caching** - LRU cache for frequently accessed documents
- **Sharding** - Hash-based sharding for 4M+ file collections
- **Lazy Indexing** - Create indices on first query, reuse thereafter
- **Write Coalescing** - Batch multiple writes to reduce fsync calls

---

## Deployment Options

### Single Machine

```bash
# Initialize store
cyberpath-sentinel init --path /var/cyberpath

# Run server
cyberpath-sentinel serve --path /var/cyberpath --port 8080
```

### Replicated Cluster (Git-backed)

```bash
# Primary node
git init --bare /data/cyberpath.git
cyberpath-sentinel serve --path /data/cyberpath --git-push origin main

# Secondary node
git clone /data/cyberpath.git /data/cyberpath
cyberpath-sentinel serve --path /data/cyberpath --git-pull origin main
```

### Encrypted Cloud Storage

```bash
# Backup to S3 with encryption
cyberpath-sentinel backup --path /data --s3-bucket compliance-backups --encryption AES256
```

---

## Documentation

- **[Implementation Plan](./IMPLEMENTATION_PLAN.md)** - Detailed architecture, pain points, solutions, and roadmap
- **[API Reference](./docs/api.md)** - Complete API documentation
- **[Security Guide](./docs/security.md)** - Encryption, ACLs, and compliance
- **[Deployment Guide](./docs/deployment.md)** - Production deployment patterns
- **[Contributing](./CONTRIBUTING.md)** - How to contribute to the project

---

## Contributing

We welcome contributions! This is an ambitious project, and we need help with:

- Core DBMS features (transactions, indexing, caching)
- Encryption and security implementations
- Performance optimization and benchmarking
- Documentation and tutorials
- Real-world use case implementations

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

---

## License

This project is licensed under the **Apache 2.0 License** - see the [LICENSE](./LICENSE) file for details.

---

## Vision

Cyberpath Sentinel is building the **gold standard for transparent, auditable data storage**. We're not trying to
replace PostgreSQL or MongoDB, we're creating something new for organizations that choose accountability over speed, and
transparency over convenience.

In five years, we want Cyberpath Sentinel to be synonymous with:

- **Compliance as Code** - Your data is your audit trail
- **Security by Design** - Every document is inspectable and verifiable
- **Trust Infrastructure** - The UNIX of data storage
- **Edge Intelligence** - Secure, offline-first data stores on every device

---

## Contact & Community

- **GitHub** - [cyberpath-sentinel](https://github.com/cyberpath-HQ/sentinel)
- **Discussions** - [GitHub Discussions](https://github.com/cyberpath-HQ/sentinel/discussions)
- **Issues** - [GitHub Issues](https://github.com/cyberpath-HQ/sentinel/issues)
- **Security** - [Security Policy](./SECURITY.md)

---

**Made with ❤️ for security teams, compliance officers, and developers who believe data should be transparent.**
