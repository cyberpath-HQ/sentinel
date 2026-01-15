---
title: Introduction
description:
  Learn about Cyberpath Sentinel, a filesystem-backed document DBMS designed for trust, transparency, and compliance.
section: Getting Started
order: 1
keywords: ["introduction", "overview", "filesystem", "document database"]
related: ["installation", "quick-start"]
---

Cyberpath Sentinel is a **filesystem-backed document DBMS** written in Rust that stores all data as files on disk. In a
world where databases hide your data behind proprietary formats and complex abstractions, Sentinel takes a radically
different approach: every piece of data is stored as a plain JSON file that you can inspect, edit, version, and audit
with standard tools.

Modern databases prioritize raw throughput and query performance. Sentinel prioritizes **trust, transparency, and
compliance**. If your organization needs to know exactly what data you have, who changed it, and when, then, Sentinel gives you
that visibility without any black boxes.

## Why Sentinel?

Traditional databases store your data in opaque binary formats. You need specialized tools to inspect it,
vendor-specific utilities to back it up, and proprietary knowledge to troubleshoot issues. When auditors come knocking,
you need complex export procedures to show them what you have.

Sentinel flips this model entirely. Your data lives as files on a filesystem:

- **Inspect with `cat`**: Every document is pretty-printed JSON
- **Backup with `tar` or `rsync`**: Standard UNIX tools work perfectly
- **Version with `git`**: Your entire database can be a Git repository
- **Audit with `grep`**: Find what you need with familiar commands
- **Delete with `rm`**: GDPR right-to-delete is literally a file deletion

This approach brings unprecedented transparency to your data. No more wondering what's inside your database—you can see
it all, right there in the filesystem.

## Perfect For

Sentinel shines in environments where **auditability** and **compliance** are non-negotiable:

- **Audit Logs**: Every entry is a file, every change is trackable with Git
- **Certificate Management**: Secure, inspectable, protected by OS-level ACLs
- **Compliance Rules & Policies**: GDPR, SOC 2, HIPAA compliance built into the architecture
- **Encryption Key Management**: Keys stored as encrypted files with filesystem security
- **Regulatory Reporting**: All data is immediately forensic-friendly
- **Edge Devices & Disconnected Systems**: No server required, works offline with Git sync
- **Zero-Trust Infrastructure**: Inspect everything before trusting it

## Not For

Sentinel makes honest trade-offs. It's **not designed** for:

- Real-time bidding systems requiring microsecond latency, and will never compete there
- High-frequency trading platforms with millions of transactions per second
- Streaming analytics pipelines processing continuous data flows
- Multi-million row transactional systems requiring complex joins

If you need raw throughput over transparency, traditional databases like PostgreSQL, MongoDB, or DuckDB are better
choices. Sentinel is for when you need to **trust your data completely** and raw speed is secondary.

## Architecture Overview

Sentinel follows a simple, hierarchical model:

```text
Store
└── Collection (directory)
    └── Document (JSON file)
```

A **Store** is the top-level container that manages all your collections. Each **Collection** is represented as a
directory containing your documents. Every **Document** is an individual JSON file with embedded metadata including
versioning, timestamps, cryptographic hashes, and optional digital signatures.

```text
data/
├── users/
│   ├── user-123.json
│   ├── user-456.json
│   └── user-789.json
├── audit_logs/
│   ├── audit-2026-01-01.json
│   └── audit-2026-01-02.json
└── certificates/
    ├── cert-abc123.json
    └── cert-def456.json
```

## Core Design Principles

Sentinel is built on five fundamental principles:

1. **Filesystem is the Database**: We leverage the reliability and tooling of battle-tested filesystems rather than
   inventing our own storage layer.

2. **Transparency by Default**: Every document is human-readable JSON. There are no binary formats, no compression, no
   obfuscation by default.

3. **Security First**: Documents are automatically hashed for integrity verification. Optional Ed25519 signatures
   provide tamper-evident storage.

4. **UNIX Philosophy**: Do one thing well. Compose with standard tools. Work the way developers expect.

5. **Zero Lock-In**: Your data is pure JSON. You can migrate to any other system at any time using standard tools.

## Getting Started

Ready to try Sentinel? Here's your path forward:

1. **[Installation](/docs/installation)**: Add Sentinel to your Rust project
2. **[Quick Start](/docs/quick-start)**: Create your first store and documents in minutes
3. **[Core Concepts](/docs/store)**: Understand stores, collections, and documents

Sentinel is open source under the Apache 2.0 license. You can find the source code, report issues, and contribute on
[GitHub](https://github.com/cyberpath-HQ/sentinel).
