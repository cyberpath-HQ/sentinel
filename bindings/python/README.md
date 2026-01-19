# Cyberpath Sentinel Python Bindings

Python bindings for Cyberpath Sentinel, a filesystem-backed document database written in Rust.

## Installation

The Python extension is built as part of the Rust workspace. To use it:

1. Build the extension:
```bash
cargo build --release -p sentinel-python
```

2. The extension will be available as `sentinel.so` in the `target/release/` directory.

## Usage

```python
import sentinel

# Create a new database store
store = sentinel.Store.new("/path/to/database")

# Create or access a collection
users = store.collection("users")

# Insert documents
user_data = {
    "name": "Alice",
    "email": "alice@example.com",
    "age": 30
}
users.insert("user-123", user_data)

# Retrieve documents
doc = users.get("user-123")
if doc:
    print(f"User: {doc.data}")
    print(f"Created: {doc.created_at}")

# Count documents
count = users.count()
print(f"Total users: {count}")

# List collections
collections = store.list_collections()
print(f"Collections: {collections}")
```

## Features

### Store
- `Store.new(path, passphrase=None)` - Create a new database store
- `store.collection(name)` - Get or create a collection
- `store.list_collections()` - List all collections in the store

### Collection
- `collection.name()` - Get the collection name
- `collection.insert(id, data)` - Insert or update a document
- `collection.get(id)` - Retrieve a document by ID
- `collection.count()` - Count documents in the collection

### Document
- `document.id` - Document identifier
- `document.version` - Document version
- `document.created_at` - Creation timestamp (ISO string)
- `document.updated_at` - Last update timestamp (ISO string)
- `document.hash` - Document hash
- `document.data` - Document data (Python dict)

## Data Types

The Python bindings support all standard JSON data types:
- Strings, numbers, booleans
- Objects (Python dicts)
- Arrays (Python lists)
- null values

## Error Handling

All operations return appropriate Python exceptions on failure. Common errors include:
- Invalid collection/document IDs
- I/O errors
- JSON serialization errors

## Thread Safety

The Python bindings are thread-safe. Multiple threads can access the same store and collections concurrently.