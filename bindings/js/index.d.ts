/**
 * Cyberpath Sentinel - TypeScript Definitions
 * 
 * TypeScript type definitions for the Cyberpath Sentinel database bindings.
 * Provides full type safety for all operations.
 * 
 * @module @cyberpath/sentinel
 */

export {};

/**
 * Operator types for query filters
 */
export enum Operator {
  Equals = 'Equals',
  GreaterThan = 'GreaterThan',
  LessThan = 'LessThan',
  GreaterOrEqual = 'GreaterOrEqual',
  LessOrEqual = 'LessOrEqual',
  Contains = 'Contains',
  StartsWith = 'StartsWith',
  EndsWith = 'EndsWith',
  In = 'In',
  Exists = 'Exists'
}

/**
 * Sort order options for query results
 */
export enum SortOrder {
  Ascending = 'Ascending',
  Descending = 'Descending'
}

/**
 * Verification mode for document integrity checks
 */
export enum VerificationMode {
  Strict = 'Strict',
  Warn = 'Warn',
  Silent = 'Silent'
}

/**
 * Options for controlling verification behavior
 */
export interface VerificationOptions {
  verifySignature: boolean;
  verifyHash: boolean;
  signatureVerificationMode: VerificationMode;
  emptySignatureMode: VerificationMode;
  hashVerificationMode: VerificationMode;
}

/**
 * A filter condition for querying documents
 */
export interface Filter {
  field: string;
  operator: Operator;
  value: any;
}

/**
 * Sort specification for query results
 */
export interface Sort {
  field: string;
  order: SortOrder;
}

/**
 * Query object for structured queries
 */
export interface Query {
  filters: Filter[];
  sort?: Sort;
  limit?: number;
  offset?: number;
  projection?: string[];
}

/**
 * Result of executing a query
 */
export interface QueryResult {
  documents: Document[];
  totalCount: number;
  executionTimeMs: number;
}

/**
 * Represents a document in the database
 */
export interface Document {
  id: string;
  version: number;
  createdAt: string;
  updatedAt: string;
  hash: string;
  signature: string;
  data: any;
}

/**
 * Options for getting documents with verification
 */
export interface GetOptions {
  verifySignature?: boolean;
  verifyHash?: boolean;
  signatureVerificationMode?: VerificationMode;
  emptySignatureMode?: VerificationMode;
  hashVerificationMode?: VerificationMode;
}

/**
 * Sentinel Store - the main entry point for database operations
 */
export class SentinelStore {
  /**
   * Create a new Sentinel store at the specified path
   * @param path - The filesystem path where the store will be created
   * @param passphrase - Optional passphrase for encryption key management
   */
  static create(path: string, passphrase?: string): Promise<SentinelStore>;
  
  /**
   * Get or create a collection by name
   * @param name - The name of the collection
   */
  collection(name: string): Promise<SentinelCollection>;
  
  /**
   * Delete a collection and all its documents
   * @param name - The name of the collection to delete
   */
  deleteCollection(name: string): Promise<void>;
  
  /**
   * List all collections in the store
   */
  listCollections(): Promise<string[]>;
  
  /**
   * Close the store and release resources
   */
  close(): Promise<void>;
}

/**
 * Sentinel Collection - manages documents in a namespace
 */
export class SentinelCollection {
  /**
   * The name of the collection
   */
  readonly name: string;
  
  /**
   * Insert a document into the collection
   * @param id - The unique identifier for the document
   * @param data - The JSON data to store
   */
  insert(id: string, data: any): Promise<void>;
  
  /**
   * Get a document by ID
   * @param id - The document ID
   * @param options - Optional verification options
   */
  get(id: string, options?: GetOptions): Promise<SentinelDocument | null>;
  
  /**
   * Delete a document from the collection (soft delete)
   * @param id - The document ID to delete
   */
  delete(id: string): Promise<void>;
  
  /**
   * Count the total number of documents in the collection
   */
  count(): Promise<number>;
  
  /**
   * Update an existing document
   * @param id - The document ID
   * @param data - The new JSON data
   */
  update(id: string, data: any): Promise<void>;
  
  /**
   * Upsert a document (insert or update)
   * @param id - The document ID
   * @param data - The JSON data
   * @returns true if document was inserted, false if updated
   */
  upsert(id: string, data: any): Promise<boolean>;
  
  /**
   * Bulk insert multiple documents
   * @param documents - Array of documents with id and data properties
   */
  bulkInsert(documents: Array<{ id: string; data: any }>): Promise<void>;
  
  /**
   * List all document IDs in the collection
   */
  list(): Promise<string[]>;
  
  /**
   * Get all documents in the collection
   * @param options - Optional verification options
   */
  all(options?: GetOptions): Promise<SentinelDocument[]>;
  
  /**
   * Execute a structured query against the collection
   * @param query - The query to execute
   * @param options - Optional verification options
   */
  query(query: Query, options?: GetOptions): Promise<QueryResult>;
  
  /**
   * Async iterator for iterating over all documents
   */
  [Symbol.asyncIterator](): AsyncIterator<SentinelDocument>;
}

/**
 * Sentinel Document - represents a document in the database
 */
export class SentinelDocument {
  /**
   * The unique identifier of the document
   */
  readonly id: string;
  
  /**
   * The version of the document
   */
  readonly version: number;
  
  /**
   * The creation timestamp (RFC3339 format)
   */
  readonly createdAt: string;
  
  /**
   * The last update timestamp (RFC3339 format)
   */
  readonly updatedAt: string;
  
  /**
   * The hash of the document data
   */
  readonly hash: string;
  
  /**
   * The signature of the document data
   */
  readonly signature: string;
  
  /**
   * The JSON data of the document
   */
  readonly data: any;
  
  /**
   * Convert document to JSON object
   */
  toJSON(): Document;
}

/**
 * QueryBuilder - fluent interface for building queries
 */
export interface QueryBuilder {
  /**
   * Add a filter condition
   * @param field - The field name to filter on
   * @param operator - The comparison operator
   * @param value - The value to compare against
   */
  filter(field: string, operator: Operator, value: any): QueryBuilder;
  
  /**
   * Add a logical AND condition
   * @param filter - The filter to AND with
   */
  and(filter: Filter): QueryBuilder;
  
  /**
   * Add a logical OR condition
   * @param filter - The filter to OR with
   */
  or(filter: Filter): QueryBuilder;
  
  /**
   * Set the sort order
   * @param field - The field to sort by
   * @param order - The sort order
   */
  sort(field: string, order: SortOrder): QueryBuilder;
  
  /**
   * Set the maximum number of results
   * @param limit - Maximum number of documents to return
   */
  limit(limit: number): QueryBuilder;
  
  /**
   * Set the number of results to skip
   * @param offset - Number of documents to skip
   */
  offset(offset: number): QueryBuilder;
  
  /**
   * Set field projection (select specific fields)
   * @param fields - List of field names to include
   */
  projection(fields: string[]): QueryBuilder;
  
  /**
   * Build the query object
   */
  build(): Query;
}

/**
 * Create a new QueryBuilder instance
 */
export function createQueryBuilder(): QueryBuilder;

/**
 * Low-level bindings (for advanced use cases)
 */
export interface Bindings {
  isWasm: boolean;
  Store: any;
  Collection: any;
  Document: any;
  QueryBuilder: any;
  Operator: typeof Operator;
  SortOrder: typeof SortOrder;
  VerificationMode: typeof VerificationMode;
  VerificationOptions: any;
}

/**
 * Access low-level bindings
 */
export const bindings: Bindings;
