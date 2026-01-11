<p align="center">
  <picture>
    <source srcset="./.assets/logo-white.svg" media="(prefers-color-scheme: dark)" />
    <source srcset="./.assets/logo.svg" media="(prefers-color-scheme: light)" />
    <img src="./.assets/logo.svg" alt="Sentinel Logo" height="64"/>
  </picture>
</p>

[![Cyberpath](https://img.shields.io/badge/Cyberpath-project-blue)](https://cyberpath-hq.com)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-green.svg)](LICENSE.md)
[![codecov](https://codecov.io/gh/cyberpath-HQ/sentinel/branch/main/graph/badge.svg?token=YOUR_TOKEN)](https://codecov.io/gh/cyberpath-HQ/sentinel)

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
cargo install cyberpath-sentinel
# or
git clone https://github.com/cyberpath-HQ/cyberpath-sentinel
cd cyberpath-sentinel
cargo build --release
```

### Basic Usage

```rust
use cyberpath_sentinel::Store;

// Create a store (creates data/ folder)
let store = Store::new("./data").expect("Failed to create store");

// Create a collection (creates data/users/ folder)
let users = store.collection("users").expect("Failed to create collection");

// Insert a document (creates data/users/user-123.json)
users.insert("user-123", json!({
    "name": "Alice",
    "email": "alice@example.com",
    "role": "admin",
    "created_at": "2026-01-11T12:00:00Z"
})).expect("Failed to insert");

// Query documents
let user = users.get("user-123").expect("Failed to get");
println!("{}", user);

// List all documents
let all_users = users.list().expect("Failed to list");
for user_id in all_users {
    println!("{}", user_id);
}

// Update a document
users.update("user-123", json!({
    "name": "Alice",
    "email": "alice.smith@example.com",
    "role": "admin",
    "created_at": "2026-01-11T12:00:00Z"
})).expect("Failed to update");

// Delete a document (creates data/users/.deleted/user-123.json)
users.delete("user-123").expect("Failed to delete");

// Query with filters
let admins = users.filter(|doc| {
    doc["role"].as_str().unwrap_or("") == "admin"
}).expect("Failed to filter");
```

### Folder Structure

```
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

## 📋 Features & Roadmap

### Core Features (Phase 1)

- [ ] Document storage as files
- [ ] Collections (folder-based)
- [ ] CRUD operations (Create, Read, Update, Delete)
- [ ] JSON document format
- [ ] Query engine with filtering
- [ ] Transaction support (atomic operations)
- [ ] File-level encryption

### Advanced Features (Phase 2)

- [ ] Write-Ahead Logging (WAL) for durability
- [ ] File locking for concurrent writes
- [ ] In-memory caching and LRU eviction
- [ ] Indexing strategies (lazy indices, hash indices)
- [ ] Full-text search
- [ ] Replication and sync (Git integration)

### Enterprise Features (Phase 3)

- [ ] Multi-version MVCC (Multi-Version Concurrency Control)
- [ ] Backup and restore utilities
- [ ] Compliance reporting dashboards
- [ ] Audit trail generation
- [ ] Encryption at rest (AES-256 or XChaCha20-Poly1305)
- [ ] Access control lists (ACLs)
- [ ] Time-series data optimization

### Game-Changing Features (Phase 4)

- [ ] Distributed consensus (Raft-based replication)
- [ ] Content-addressable storage (like Git)
- [ ] Merkle tree verification for integrity
- [ ] Automated compliance audits
- [ ] Zero-knowledge proof capabilities
- [ ] Decentralized document signing
- [ ] Temporal queries (valid-at-timestamp)

---

## Architecture

### Design Principles

1. **Filesystem is the Database** - Leverage OS reliability and tooling
2. **Immutability by Default** - Audit trails, append-only logs, deletions are soft
3. **Security First** - File permissions, encryption, cryptographic verification
4. **UNIX Philosophy** - Do one thing well, compose with standard tools
5. **Zero External Dependencies** - Works offline, on edge devices

### Core Components

```
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

This project is licensed under the **MIT License** - see the [LICENSE](./LICENSE) file for details.

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
