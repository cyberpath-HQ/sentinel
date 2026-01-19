/**
 * Cyberpath Sentinel - JavaScript/TypeScript bindings for Node.js
 *
 * This module provides a unified API for Cyberpath Sentinel using native N-API bindings.
 *
 * @module @cyberpath/sentinel
 */

'use strict';

const native = require('./native/sentinel_js.node');

const Store = native.Store;
const Collection = native.Collection;
const Document = native.Document;
const QueryBuilder = native.QueryBuilder;
const Operator = native.Operator;
const SortOrder = native.SortOrder;
const VerificationMode = native.VerificationMode;
const VerificationOptions = native.VerificationOptions;

exports.isWasm = false;
exports.isNative = true;
exports.Store = Store;
exports.Collection = Collection;
exports.Document = Document;
exports.QueryBuilder = QueryBuilder;
exports.Operator = Operator;
exports.SortOrder = SortOrder;
exports.VerificationMode = VerificationMode;
exports.VerificationOptions = VerificationOptions;

class SentinelStore {
  constructor(store) {
    this._store = store;
  }

  static async create(path, passphrase = undefined) {
    const store = await Store.create(path, passphrase);
    return new SentinelStore(store);
  }

  async collection(name) {
    const collection = await this._store.collection(name);
    return new SentinelCollection(collection);
  }

  async deleteCollection(name) {
    return this._store.deleteCollection(name);
  }

  async listCollections() {
    return this._store.listCollections();
  }

  async close() {
    if (typeof this._store.close === 'function') {
      return this._store.close();
    }
  }
}

class SentinelCollection {
  constructor(collection) {
    this._collection = collection;
  }

  get name() {
    return this._collection.name;
  }

  async insert(id, data) {
    return this._collection.insert(id, data);
  }

  async get(id, options = undefined) {
    if (options) {
      const jsOptions = new VerificationOptions();
      jsOptions.verifySignature = options.verifySignature ?? true;
      jsOptions.verifyHash = options.verifyHash ?? true;
      jsOptions.signatureVerificationMode = options.signatureVerificationMode ?? VerificationMode.Strict;
      jsOptions.emptySignatureMode = options.emptySignatureMode ?? VerificationMode.Warn;
      jsOptions.hashVerificationMode = options.hashVerificationMode ?? VerificationMode.Strict;
      const result = await this._collection.getWithVerification(id, jsOptions);
      return result ? new SentinelDocument(result) : null;
    }
    const result = await this._collection.get(id);
    return result ? new SentinelDocument(result) : null;
  }

  async delete(id) {
    return this._collection.delete(id);
  }

  async count() {
    return this._collection.count();
  }

  async update(id, data) {
    return this._collection.update(id, data);
  }

  async upsert(id, data) {
    return this._collection.upsert(id, data);
  }

  async bulkInsert(documents) {
    const docs = documents.map(doc => ({
      id: doc.id,
      data: doc.data
    }));
    return this._collection.bulkInsert(docs);
  }

  async list() {
    return this._collection.list();
  }

  async all(options = undefined) {
    if (options) {
      const jsOptions = new VerificationOptions();
      jsOptions.verifySignature = options.verifySignature ?? true;
      jsOptions.verifyHash = options.verifyHash ?? true;
      jsOptions.signatureVerificationMode = options.signatureVerificationMode ?? VerificationMode.Strict;
      jsOptions.emptySignatureMode = options.emptySignatureMode ?? VerificationMode.Warn;
      jsOptions.hashVerificationMode = options.hashVerificationMode ?? VerificationMode.Strict;
      const results = await this._collection.allWithVerification(jsOptions);
      return results.map(doc => new SentinelDocument(doc));
    }
    const results = await this._collection.all();
    return results.map(doc => new SentinelDocument(doc));
  }

  async query(query, options = undefined) {
    const queryBuilder = new QueryBuilder();
    
    if (query.filters) {
      for (const filter of query.filters) {
        queryBuilder.filter(filter.field, filter.operator, filter.value);
      }
    }
    
    if (query.sort) {
      queryBuilder.sort(query.sort.field, query.sort.order);
    }
    
    if (query.limit) {
      queryBuilder.limit(query.limit);
    }
    
    if (query.offset) {
      queryBuilder.offset(query.offset);
    }
    
    if (query.projection) {
      queryBuilder.projection(query.projection);
    }
    
    const builtQuery = queryBuilder.build();
    
    if (options) {
      const jsOptions = new VerificationOptions();
      jsOptions.verifySignature = options.verifySignature ?? true;
      jsOptions.verifyHash = options.verifyHash ?? true;
      jsOptions.signatureVerificationMode = options.signatureVerificationMode ?? VerificationMode.Strict;
      jsOptions.emptySignatureMode = options.emptySignatureMode ?? VerificationMode.Warn;
      jsOptions.hashVerificationMode = options.hashVerificationMode ?? VerificationMode.Strict;
      const result = await this._collection.queryWithVerification(builtQuery, jsOptions);
      return {
        documents: result.documents.map(doc => new SentinelDocument(doc)),
        totalCount: result.totalCount,
        executionTimeMs: result.executionTimeMs
      };
    }
    
    const result = await this._collection.query(builtQuery);
    return {
      documents: result.documents.map(doc => new SentinelDocument(doc)),
      totalCount: result.totalCount,
      executionTimeMs: result.executionTimeMs
    };
  }

  [Symbol.asyncIterator]() {
    return this._allAsyncIterator();
  }

  async *_allAsyncIterator() {
    const docs = await this.all();
    for (const doc of docs) {
      yield doc;
    }
  }
}

class SentinelDocument {
  constructor(document) {
    this._document = document;
    this.id = document.id;
    this.version = document.version;
    this.createdAt = document.createdAt;
    this.updatedAt = document.updatedAt;
    this.hash = document.hash;
    this.signature = document.signature;
    this.data = document.data;
  }

  toJSON() {
    return {
      id: this.id,
      version: this.version,
      createdAt: this.createdAt,
      updatedAt: this.updatedAt,
      hash: this.hash,
      signature: this.signature,
      data: this.data
    };
  }
}

function createQueryBuilder() {
  return {
    filters: [],
    sort: null,
    limit: null,
    offset: null,
    projection: null,
    
    filter(field, operator, value) {
      this.filters.push({ field, operator, value });
      return this;
    },
    
    and(filter) {
      if (this.filters.length > 0) {
        const last = this.filters.pop();
        this.filters.push({
          type: 'and',
          children: [last, filter]
        });
      } else {
        this.filters.push(filter);
      }
      return this;
    },
    
    or(filter) {
      if (this.filters.length > 0) {
        const last = this.filters.pop();
        this.filters.push({
          type: 'or',
          children: [last, filter]
        });
      } else {
        this.filters.push(filter);
      }
      return this;
    },
    
    sort(field, order) {
      this.sort = { field, order };
      return this;
    },
    
    limit(n) {
      this.limit = n;
      return this;
    },
    
    offset(n) {
      this.offset = n;
      return this;
    },
    
    projection(fields) {
      this.projection = fields;
      return this;
    },
    
    build() {
      return {
        filters: this.filters,
        sort: this.sort,
        limit: this.limit,
        offset: this.offset,
        projection: this.projection
      };
    }
  };
}

exports.SentinelStore = SentinelStore;
exports.SentinelCollection = SentinelCollection;
exports.SentinelDocument = SentinelDocument;
exports.createQueryBuilder = createQueryBuilder;
