---
title: Quick Start
description: Create your first Sentinel store, collection, and documents in minutes.
section: Getting Started
order: 3
keywords: ["quick start", "tutorial", "getting started", "example"]
related: ["installation", "store", "collection", "document"]
---

This guide walks you through creating your first Sentinel database in just a few minutes. By the end, you'll understand
how to create stores, manage collections, and work with documents.

## Your First Store

A Store is the top-level container for all your data. It maps to a directory on your filesystem:

```rust
use sentinel_dbms::Store;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a store at ./my-database
    // The directory will be created if it doesn't exist
    let store = Store::new("./my-database", None).await?;

    println!("Store created at ./my-database");
    Ok(())
}
```

The second parameter (`None`) indicates we're not using a passphrase for signing. We'll cover signed documents later.

After running this code, you'll see a new directory:

```text
my-database/
```

## Creating a Collection

Collections are namespaces for related documents. They're represented as subdirectories within your store:

```rust
use sentinel_dbms::Store;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Store::new("./my-database", None).await?;

    // Get or create a "users" collection
    let users = store.collection("users").await?;

    println!("Collection 'users' ready!");
    Ok(())
}
```

Your filesystem now looks like:

```text
my-database/
└── data/
    └── users/
```

## Inserting Documents

Documents are JSON objects stored as individual files. Use `insert` to create a new document:

```rust
use sentinel_dbms::Store;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Store::new("./my-database", None).await?;
    let users = store.collection("users").await?;

    // Insert a document with ID "alice"
    users.insert("alice", json!({
        "name": "Alice Johnson",
        "email": "alice@example.com",
        "role": "admin",
        "department": "Engineering"
    })).await?;

    println!("Document 'alice' created!");
    Ok(())
}
```

This creates a file at `my-database/data/users/alice.json` containing:

```json
{
  "id": "alice",
  "version": 1,
  "created_at": "2026-01-15T12:00:00Z",
  "updated_at": "2026-01-15T12:00:00Z",
  "hash": "a1b2c3d4e5f6789...",
  "signature": "",
  "data": {
    "name": "Alice Johnson",
    "email": "alice@example.com",
    "role": "admin",
    "department": "Engineering"
  }
}
```

Notice how Sentinel automatically adds metadata: the document ID, version number, timestamps, and a BLAKE3 hash for
integrity verification.

## Retrieving Documents

Use `get` to retrieve a document by its ID:

```rust
use sentinel_dbms::Store;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Store::new("./my-database", None).await?;
    let users = store.collection("users").await?;

    // Retrieve the document
    if let Some(doc) = users.get("alice").await? {
        println!("Found user: {}", doc.data()["name"]);
        println!("Email: {}", doc.data()["email"]);
        println!("Created at: {}", doc.created_at());
        println!("Hash: {}", doc.hash());
    } else {
        println!("User not found");
    }

    Ok(())
}
```

The `get` method returns `Option<Document>`. If the document doesn't exist, you get `None` instead of an error.

## Updating Documents

The `update` method replaces a document's contents while preserving the file location:

```rust
use sentinel_dbms::Store;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Store::new("./my-database", None).await?;
    let users = store.collection("users").await?;

    // Update Alice's information
    users.update("alice", json!({
        "name": "Alice Johnson",
        "email": "alice.johnson@example.com",  // New email
        "role": "senior_admin",                 // Promoted!
        "department": "Engineering"
    })).await?;

    println!("Document updated!");
    Ok(())
}
```

The updated document gets a new timestamp and hash, allowing you to track when changes occurred.

## Deleting Documents

Use `delete` to remove a document:

```rust
use sentinel_dbms::Store;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Store::new("./my-database", None).await?;
    let users = store.collection("users").await?;

    // Delete the document
    users.delete("alice").await?;

    println!("Document deleted!");
    Ok(())
}
```

The `delete` operation is idempotent—deleting a non-existent document succeeds without error.

## Adding Signatures

For tamper-evident storage, create a store with a passphrase. This generates a signing key that will be used to sign all
documents:

```rust
use sentinel_dbms::Store;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a store with signing enabled
    let store = Store::new("./secure-database", Some("my-secret-passphrase")).await?;
    let secrets = store.collection("secrets").await?;

    // This document will be signed automatically
    secrets.insert("api-key", json!({
        "service": "payment-gateway",
        "key": "sk_live_abc123",
        "created_by": "alice"
    })).await?;

    println!("Signed document created!");
    Ok(())
}
```

The document now includes an Ed25519 signature:

```json
{
  "id": "api-key",
  "version": 1,
  "created_at": "2026-01-15T12:00:00Z",
  "updated_at": "2026-01-15T12:00:00Z",
  "hash": "a1b2c3d4e5f6789...",
  "signature": "ed25519:xyz789abc...",
  "data": {
    "service": "payment-gateway",
    "key": "sk_live_abc123",
    "created_by": "alice"
  }
}
```

## Complete Example

Here's a complete example combining everything we've learned:

```rust
use sentinel_dbms::Store;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a store with signing
    let store = Store::new("./company-data", Some("secret123")).await?;

    // Create collections for different data types
    let employees = store.collection("employees").await?;
    let audit_logs = store.collection("audit_logs").await?;

    // Add some employees
    employees.insert("emp-001", json!({
        "name": "Alice Johnson",
        "title": "Software Engineer",
        "department": "Engineering",
        "start_date": "2024-01-15"
    })).await?;

    employees.insert("emp-002", json!({
        "name": "Bob Smith",
        "title": "Product Manager",
        "department": "Product",
        "start_date": "2023-06-01"
    })).await?;

    // Log an audit event
    audit_logs.insert("2026-01-15-001", json!({
        "timestamp": "2026-01-15T14:30:00Z",
        "action": "employee_created",
        "actor": "admin@company.com",
        "target": "emp-001",
        "details": "New employee onboarding"
    })).await?;

    // Read back an employee
    if let Some(alice) = employees.get("emp-001").await? {
        println!("Employee: {}", alice.data()["name"]);
        println!("Hash: {}", alice.hash());
        println!("Signature: {}", alice.signature());
    }

    println!("\n✓ Database created successfully!");
    println!("  Check ./company-data to see your files!");

    Ok(())
}
```

After running this, explore your filesystem:

```bash
ls -la ./company-data/data/
cat ./company-data/data/employees/emp-001.json
```

## Next Steps

Now that you understand the basics, dive deeper into:

- **[Store](/docs/store)** — Learn about store configuration and management
- **[Collection](/docs/collection)** — Master collection operations and validation
- **[Document](/docs/document)** — Understand document structure and metadata
- **[Cryptography](/docs/cryptography)** — Explore hashing, signing, and encryption options
