/**
 * Cyberpath Sentinel - ESM Example
 * 
 * Example using ES module syntax
 */

import { SentinelStore, SentinelCollection, SentinelDocument, createQueryBuilder, Operator, SortOrder, VerificationMode, bindings } from '@cyberpath/sentinel';

console.log('Running Sentinel ESM example...');
console.log('Environment:', bindings.isWasm ? 'WebAssembly (Browser)' : 'Native (Node.js)');

async function example() {
  const tempDir = '/tmp';
  const storePath = `${tempDir}/sentinel-esm-example-${Date.now()}`;
  
  console.log(`\n1. Creating store at: ${storePath}`);
  const store = await SentinelStore.create(storePath, 'example-passphrase');
  
  console.log('\n2. Creating collections...');
  const users = await store.collection('users');
  
  console.log('\n3. Inserting documents...');
  await users.insert('user-1', { name: 'Alice', age: 30 });
  await users.insert('user-2', { name: 'Bob', age: 25 });
  
  console.log('\n4. Getting documents...');
  const user1 = await users.get('user-1');
  console.log('User 1:', user1.data);
  
  console.log('\n5. Querying documents...');
  const query = createQueryBuilder()
    .filter('age', Operator.GreaterThan, 20)
    .sort('name', SortOrder.Ascending)
    .build();
  
  const result = await users.query(query);
  console.log(`Found ${result.documents.length} users older than 20`);
  
  console.log('\n✅ ESM Example completed successfully!');
}

example().catch(console.error);
