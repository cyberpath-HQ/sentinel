/**
 * JavaScript integration tests for sentinel-js
 * These tests run via Node.js when executing cargo test -p sentinel-js
 */

const assert = require('assert');
const path = require('path');
const fs = require('fs');

// Load the native bindings
const native = require('../native/sentinel_js.node');
const { JsStore, JsCollection, JsDocument, JsVerificationOptions, JsOperator, JsSortOrder, JsVerificationMode } = native;

describe('SentinelStore', function() {
    this.timeout(10000);
    
    let testDir;
    let store;

    before(async () => {
        testDir = fs.mkdtempSync(path.join(require('os').tmpdir(), 'sentinel-test-'));
    });

    after(() => {
        if (testDir && fs.existsSync(testDir)) {
            fs.rmSync(testDir, { recursive: true, force: true });
        }
    });

    it('should create a new store', async () => {
        store = await JsStore.create(testDir);
        assert.ok(store, 'Store should be created');
    });

    it('should list empty collections initially', async () => {
        const collections = await store.listCollections();
        assert.ok(Array.isArray(collections), 'Should return an array');
        assert.strictEqual(collections.length, 0, 'Should have no collections');
    });

    it('should create a collection', async () => {
        const collection = await store.collection('test_collection');
        assert.ok(collection, 'Collection should be created');
        assert.strictEqual(collection.name, 'test_collection');
    });

    it('should list created collections', async () => {
        const collections = await store.listCollections();
        assert.ok(collections.includes('test_collection'), 'Collection should be in list');
    });

    it('should delete a collection', async () => {
        await store.deleteCollection('test_collection');
        const collections = await store.listCollections();
        assert.ok(!collections.includes('test_collection'), 'Collection should be deleted');
    });
});

describe('SentinelCollection', function() {
    this.timeout(10000);
    
    let testDir;
    let store;
    let collection;

    before(async () => {
        testDir = fs.mkdtempSync(path.join(require('os').tmpdir(), 'sentinel-test-'));
        store = await JsStore.create(testDir);
        collection = await store.collection('documents');
        
        // Clean up any existing documents from previous test runs
        const existingDocs = await collection.list();
        for (const id of existingDocs) {
            await collection.delete(id).catch(() => {});
        }
    });

    after(() => {
        if (testDir && fs.existsSync(testDir)) {
            fs.rmSync(testDir, { recursive: true, force: true });
        }
    });

    describe('CRUD Operations', function() {
        it('should insert a document', async () => {
            await collection.insert('doc-1', { name: 'Test Document', value: 42 });
            const doc = await collection.get('doc-1');
            assert.ok(doc, 'Document should exist');
            assert.strictEqual(doc.data.name, 'Test Document');
            assert.strictEqual(doc.data.value, 42);
        });

        it('should get a non-existent document', async () => {
            const doc = await collection.get('non-existent');
            assert.strictEqual(doc, null, 'Should return null for non-existent document');
        });

        it('should update a document', async () => {
            await collection.insert('doc-1', { name: 'Original' });
            await collection.update('doc-1', { name: 'Updated', value: 100 });
            const doc = await collection.get('doc-1');
            assert.strictEqual(doc.data.name, 'Updated');
            assert.strictEqual(doc.data.value, 100);
        });

        it('should delete a document', async () => {
            await collection.insert('doc-to-delete', { name: 'Will be deleted' });
            await collection.delete('doc-to-delete');
            const doc = await collection.get('doc-to-delete');
            assert.strictEqual(doc, null, 'Document should be deleted');
        });

        it('should upsert (insert)', async () => {
            const wasInsert = await collection.upsert('upsert-doc', { status: 'inserted' });
            assert.strictEqual(wasInsert, true, 'First upsert should be insert');
            const doc = await collection.get('upsert-doc');
            assert.strictEqual(doc.data.status, 'inserted');
        });

        it('should upsert (update)', async () => {
            await collection.upsert('upsert-doc', { status: 'inserted' });
            const wasInsert = await collection.upsert('upsert-doc', { status: 'updated' });
            assert.strictEqual(wasInsert, false, 'Second upsert should be update');
            const doc = await collection.get('upsert-doc');
            assert.strictEqual(doc.data.status, 'updated');
        });
    });

    describe('Bulk Operations', function() {
        before(async () => {
            // Clean up before bulk tests - delete all existing documents
            const existingDocs = await collection.list();
            for (const id of existingDocs) {
                await collection.delete(id).catch(() => {});
            }
            // Verify cleanup worked
            const countAfterCleanup = await collection.count();
            if (countAfterCleanup > 0) {
                console.log(`Warning: ${countAfterCleanup} documents still exist after cleanup`);
            }
        });

        it('should bulk insert documents', async () => {
            const documents = [
                { id: 'bulk-1', data: { index: 1 } },
                { id: 'bulk-2', data: { index: 2 } },
                { id: 'bulk-3', data: { index: 3 } }
            ];
            await collection.bulkInsert(documents);
            
            const count = await collection.count();
            assert.strictEqual(count, 3, `Should have 3 documents, got ${count}`);
        });

        it('should list document IDs', async () => {
            const ids = await collection.list();
            assert.strictEqual(ids.length, 3, `Should have 3 IDs, got ${ids.length}`);
            assert.ok(ids.includes('bulk-1'), 'Should include bulk-1');
            assert.ok(ids.includes('bulk-2'), 'Should include bulk-2');
            assert.ok(ids.includes('bulk-3'), 'Should include bulk-3');
        });

        it('should get all documents', async () => {
            const docs = await collection.all();
            assert.strictEqual(docs.length, 3, `Should have 3 documents, got ${docs.length}`);
        });

        it('should count documents', async () => {
            const count = await collection.count();
            assert.strictEqual(count, 3, `Should count 3 documents, got ${count}`);
        });
    });
});

describe('VerificationOptions', function() {
    it('should create strict verification', () => {
        const opts = JsVerificationOptions.strict();
        assert.strictEqual(opts.verifySignature, true);
        assert.strictEqual(opts.verifyHash, true);
        assert.strictEqual(opts.signatureVerificationMode, JsVerificationMode.Strict);
    });

    it('should create disabled verification', () => {
        const opts = JsVerificationOptions.disabled();
        assert.strictEqual(opts.verifySignature, false);
        assert.strictEqual(opts.verifyHash, false);
        assert.strictEqual(opts.signatureVerificationMode, JsVerificationMode.Silent);
    });

    it('should create warn verification', () => {
        const opts = JsVerificationOptions.warn();
        assert.strictEqual(opts.verifySignature, true);
        assert.strictEqual(opts.verifyHash, true);
        assert.strictEqual(opts.signatureVerificationMode, JsVerificationMode.Warn);
    });
});

console.log('Running sentinel-js integration tests...');
