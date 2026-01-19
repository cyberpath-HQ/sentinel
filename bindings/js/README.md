# Cyberpath Sentinel - JavaScript/TypeScript Bindings

JavaScript and TypeScript bindings for Cyberpath Sentinel, a filesystem-backed document database system. These bindings work in both Node.js (using native N-API bindings) and browsers (using WebAssembly).

## Features

- **Full API Coverage**: All features from the Rust `sentinel-dbms` crate are exposed
- **TypeScript Support**: Complete type definitions with generics
- **Async/Await**: All async operations maintain async behavior
- **Dual Runtime Support**: Works in Node.js and browsers
- **Module Compatibility**: Supports both CommonJS and ES modules

## Installation

### Node.js (Native Bindings)

```bash
npm install @cyberpath/sentinel
```

### Browser (WebAssembly)

```bash
npm install @cyberpath/sentinel @cyberpath/sentinel-wasm
```

## Quick Start

```typescript
import { SentinelStore, SentinelCollection, SentinelDocument, Operator, SortOrder } from '@cyberpath/sentinel';

async function main() {
  // Create a store
  const store = await SentinelStore.create('./data', 'my-passphrase');
  
  // Get a collection
  const users = await store.collection('users');
  
  // Insert a document
  await users.insert('user-1', { name: 'Alice', age: 30 });
  
  // Get a document
  const user = await users.get('user-1');
  console.log(user.data); // { name: 'Alice', age: 30 }
  
  // Query documents
  const result = await users.query({
    filters: [
      { field: 'age', operator: Operator.GreaterThan, value: 25 }
    ],
    sort: { field: 'name', order: SortOrder.Ascending },
    limit: 10
  });
  
  // Iterate over all documents
  for await (const doc of users) {
    console.log(doc.id, doc.data);
  }
}

main().catch(console.error);
```

## API Reference

### SentinelStore

The main entry point for database operations.

```typescript
class SentinelStore {
  // Factory method to create a new store
  static async create(path: string, passphrase?: string): Promise<SentinelStore>
  
  // Get or create a collection
  async collection(name: string): Promise<SentinelCollection>
  
  // Delete a collection
  async deleteCollection(name: string): Promise<void>
  
  // List all collections
  async listCollections(): Promise<string[]>
}
```

### SentinelCollection

Manages documents within a namespace.

```typescript
class SentinelCollection {
  // The collection name
  readonly name: string
  
  // Insert a document
  async insert(id: string, data: any): Promise<void>
  
  // Get a document by ID
  async get(id: string, options?: GetOptions): Promise<SentinelDocument | null>
  
  // Get with verification options
  async get(id: string, options: GetOptions): Promise<SentinelDocument | null>
  
  // Update a document (merges data)
  async update(id: string, data: any): Promise<void>
  
  // Upsert (insert or update)
  async upsert(id: string, data: any): Promise<boolean>
  
  // Delete a document
  async delete(id: string): Promise<void>
  
  // Count documents
  async count(): Promise<number>
  
  // Bulk insert
  async bulkInsert(documents: Array<{ id: string; data: any }>): Promise<void>
  
  // Get multiple documents
  async getMany(ids: string[]): Promise<Array<SentinelDocument | null>>
  
  // List all document IDs
  async list(): Promise<string[]>
  
  // Get all documents
  async all(options?: GetOptions): Promise<SentinelDocument[]>
  
  // Query documents
  async query(query: Query, options?: GetOptions): Promise<QueryResult>
  
  // Aggregate documents
  async aggregate(filters: Filter[], type: AggregationType, field?: string): Promise<any>
}
```

### SentinelDocument

Represents a document in the database.

```typescript
class SentinelDocument {
  readonly id: string
  readonly version: number
  readonly createdAt: string
  readonly updatedAt: string
  readonly hash: string
  readonly signature: string
  readonly data: any
  
  toJSON(): Document
}
```

### Query Builder

Fluent interface for building queries:

```typescript
const query = createQueryBuilder()
  .filter('age', Operator.GreaterThan, 25)
  .filter('city', Operator.Equals, 'NYC')
  .sort('name', SortOrder.Ascending)
  .limit(10)
  .offset(0)
  .projection(['name', 'age'])
  .build()
```

### Operators

- `Operator.Equals` - Exact match
- `Operator.GreaterThan` - Greater than
- `Operator.LessThan` - Less than
- `Operator.GreaterOrEqual` - Greater or equal
- `Operator.LessOrEqual` - Less or equal
- `Operator.Contains` - String contains
- `Operator.StartsWith` - String starts with
- `Operator.EndsWith` - String ends with
- `Operator.In` - Value in array
- `Operator.Exists` - Field exists

### Verification Options

Control document integrity verification:

```typescript
const options = {
  verifySignature: true,
  verifyHash: true,
  signatureVerificationMode: VerificationMode.Strict,
  emptySignatureMode: VerificationMode.Warn,
  hashVerificationMode: VerificationMode.Strict
}
```

## Environment Detection

The bindings automatically detect whether they're running in Node.js or browser:

```typescript
import { bindings } from '@cyberpath/sentinel';

if (bindings.isWasm) {
  console.log('Running in browser with WASM');
} else {
  console.log('Running in Node.js with native bindings');
}
```

## Building from Source

### Build Native Bindings (Node.js)

```bash
cd bindings/js
npm run build
```

### Build WebAssembly (Browser)

```bash
cd bindings/wasm
npm run build
```

### Build Both

```bash
cd bindings/js
npm run build:all
```

## Performance Considerations

- **Native Bindings**: Best performance for Node.js environments
- **WASM**: Required for browser environments, slightly slower but fully featured
- **Streaming**: Use async iteration for large datasets to avoid memory issues
- **Batching**: Use `bulkInsert` for inserting multiple documents

## License

Apache-2.0
