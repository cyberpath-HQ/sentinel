/**
 * Cyberpath Sentinel - Example Usage
 * 
 * This file demonstrates common usage patterns for the Sentinel database
 * in both Node.js and browser environments.
 * 
 * @example
 * 
 * Basic usage:
 * 
 * ```javascript
 * const { SentinelStore } = require('@cyberpath/sentinel');
 * 
 * async function main() {
 *   // Create a store
 *   const store = await SentinelStore.create('./data', 'my-passphrase');
 *   
 *   // Get a collection
 *   const users = await store.collection('users');
 *   
 *   // Insert a document
 *   await users.insert('user-1', { name: 'Alice', age: 30 });
 *   
 *   // Get a document
 *   const user = await users.get('user-1');
 *   console.log(user.data); // { name: 'Alice', age: 30 }
 *   
 *   // Query documents
 *   const result = await users.query({
 *     filters: [
 *       { field: 'age', operator: 'GreaterThan', value: 25 }
 *     ],
 *     sort: { field: 'name', order: 'Ascending' },
 *     limit: 10
 *   });
 *   
 *   // Iterate over all documents
 *   for await (const doc of users) {
 *     console.log(doc.id, doc.data);
 *   }
 * }
 * 
 * main().catch(console.error);
 * ```
 */

const { SentinelStore, SentinelCollection, SentinelDocument, createQueryBuilder, Operator, SortOrder, VerificationMode, bindings } = require('./dist/index.js');

console.log('Running Sentinel example...');
console.log('Environment:', bindings.isWasm ? 'WebAssembly (Browser)' : 'Native (Node.js)');

async function example() {
  const tempDir = require('os').tmpdir();
  const storePath = `${tempDir}/sentinel-example-${Date.now()}`;
  
  console.log(`\n1. Creating store at: ${storePath}`);
  const store = await SentinelStore.create(storePath, 'example-passphrase');
  
  console.log('\n2. Creating collections...');
  const users = await store.collection('users');
  const products = await store.collection('products');
  
  console.log('\n3. Inserting documents...');
  await users.insert('user-1', { name: 'Alice', email: 'alice@example.com', age: 30, city: 'NYC' });
  await users.insert('user-2', { name: 'Bob', email: 'bob@example.com', age: 25, city: 'LA' });
  await users.insert('user-3', { name: 'Charlie', email: 'charlie@example.com', age: 35, city: 'NYC' });
  
  await products.insert('prod-1', { name: 'Widget', price: 29.99, category: 'tools' });
  await products.insert('prod-2', { name: 'Gadget', price: 49.99, category: 'electronics' });
  
  console.log('\n4. Getting documents...');
  const user1 = await users.get('user-1');
  console.log('User 1:', user1.data);
  
  console.log('\n5. Counting documents...');
  const userCount = await users.count();
  console.log(`Users count: ${userCount}`);
  
  console.log('\n6. Using QueryBuilder...');
  const query = createQueryBuilder()
    .filter('city', Operator.Equals, 'NYC')
    .filter('age', Operator.GreaterThan, 25)
    .sort('name', SortOrder.Ascending)
    .limit(10)
    .build();
  
  const nyUsers = await users.query(query);
  console.log(`NYC users older than 25: ${nyUsers.documents.length}`);
  nyUsers.documents.forEach(doc => {
    console.log(`  - ${doc.data.name}, ${doc.data.age}`);
  });
  
  console.log('\n7. Getting all documents with async iteration...');
  for await (const user of users) {
    console.log(`  - ${user.id}: ${user.data.name}`);
  }
  
  console.log('\n8. Using verification options...');
  const secureUser = await users.get('user-1', {
    verifySignature: true,
    verifyHash: true,
    signatureVerificationMode: VerificationMode.Strict,
    hashVerificationMode: VerificationMode.Strict
  });
  console.log('Secure user retrieved:', !!secureUser);
  
  console.log('\n9. Upsert operation...');
  const wasInserted = await users.upsert('user-4', { name: 'David', age: 40 });
  console.log(`User 4 was inserted (not existed): ${wasInserted}`);
  
  const wasUpdated = await users.upsert('user-1', { name: 'Alice Smith', age: 31 });
  console.log(`User 1 was updated (existed): ${!wasUpdated}`);
  
  console.log('\n10. Bulk insert...');
  await users.bulkInsert([
    { id: 'user-5', data: { name: 'Eve', age: 28 } },
    { id: 'user-6', data: { name: 'Frank', age: 45 } }
  ]);
  
  console.log('\n11. Listing collections...');
  const collections = await store.listCollections();
  console.log(`Collections: ${collections.join(', ')}`);
  
  console.log('\n12. Cleanup...');
  await store.deleteCollection('products');
  const remaining = await store.listCollections();
  console.log(`Remaining collections: ${remaining.join(', ')}`);
  
  console.log('\n✅ Example completed successfully!');
}

example().catch(err => {
  console.error('Example failed:', err);
  process.exit(1);
});
