use std::time::Duration;

use wasm_bindgen::{prelude::*, JsValue};
use js_sys::JSON;
use web_sys::console;
use sentinel_dbms::{
    Collection,
    Document,
    Filter,
    Operator,
    Query,
    QueryBuilder,
    QueryResult,
    SortOrder,
    Store,
    VerificationMode,
    VerificationOptions,
};
use serde_wasm_bindgen::{from_value, to_value};
use serde_json::Value;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmOperator {
    Equals,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    In,
    Exists,
}

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmSortOrder {
    Ascending,
    Descending,
}

#[wasm_bindgen]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmVerificationMode {
    Strict,
    Warn,
    Silent,
}

impl From<WasmOperator> for Operator {
    fn from(op: WasmOperator) -> Self {
        match op {
            WasmOperator::Equals => Operator::Equals,
            WasmOperator::GreaterThan => Operator::GreaterThan,
            WasmOperator::LessThan => Operator::LessThan,
            WasmOperator::GreaterOrEqual => Operator::GreaterOrEqual,
            WasmOperator::LessOrEqual => Operator::LessOrEqual,
            WasmOperator::Contains => Operator::Contains,
            WasmOperator::StartsWith => Operator::StartsWith,
            WasmOperator::EndsWith => Operator::EndsWith,
            WasmOperator::In => Operator::In,
            WasmOperator::Exists => Operator::Exists,
        }
    }
}

impl From<WasmSortOrder> for SortOrder {
    fn from(order: WasmSortOrder) -> Self {
        match order {
            WasmSortOrder::Ascending => SortOrder::Ascending,
            WasmSortOrder::Descending => SortOrder::Descending,
        }
    }
}

impl From<WasmVerificationMode> for VerificationMode {
    fn from(mode: WasmVerificationMode) -> Self {
        match mode {
            WasmVerificationMode::Strict => VerificationMode::Strict,
            WasmVerificationMode::Warn => VerificationMode::Warn,
            WasmVerificationMode::Silent => VerificationMode::Silent,
        }
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WasmVerificationOptions {
    pub verify_signature:            bool,
    pub verify_hash:                 bool,
    pub signature_verification_mode: WasmVerificationMode,
    pub empty_signature_mode:        WasmVerificationMode,
    pub hash_verification_mode:      WasmVerificationMode,
}

#[wasm_bindgen]
impl WasmVerificationOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self::default() }

    #[wasm_bindgen]
    pub fn strict() -> Self {
        let opts = VerificationOptions::strict();
        Self::from_opts(opts)
    }

    #[wasm_bindgen]
    pub fn disabled() -> Self {
        let opts = VerificationOptions::disabled();
        Self::from_opts(opts)
    }

    #[wasm_bindgen]
    pub fn warn() -> Self {
        let opts = VerificationOptions::warn();
        Self::from_opts(opts)
    }

    fn from_opts(opts: VerificationOptions) -> Self {
        Self {
            verify_signature:            opts.verify_signature,
            verify_hash:                 opts.verify_hash,
            signature_verification_mode: opts.signature_verification_mode.into(),
            empty_signature_mode:        opts.empty_signature_mode.into(),
            hash_verification_mode:      opts.hash_verification_mode.into(),
        }
    }
}

impl Default for WasmVerificationOptions {
    fn default() -> Self {
        let opts = VerificationOptions::default();
        Self::from_opts(opts)
    }
}

impl From<WasmVerificationOptions> for VerificationOptions {
    fn from(opts: WasmVerificationOptions) -> Self {
        Self {
            verify_signature:            opts.verify_signature,
            verify_hash:                 opts.verify_hash,
            signature_verification_mode: opts.signature_verification_mode.into(),
            empty_signature_mode:        opts.empty_signature_mode.into(),
            hash_verification_mode:      opts.hash_verification_mode.into(),
        }
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WasmQueryBuilder {
    filters:    Vec<Filter>,
    sort:       Option<(String, SortOrder)>,
    limit:      Option<usize>,
    offset:     Option<usize>,
    projection: Option<Vec<String>>,
}

#[wasm_bindgen]
impl WasmQueryBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            filters:    Vec::new(),
            sort:       None,
            limit:      None,
            offset:     None,
            projection: None,
        }
    }

    pub fn filter(&mut self, field: String, op: WasmOperator, value: JsValue) -> Result<(), JsValue> {
        let value: Value = from_value(value).map_err(|e| JsValue::from_str(&format!("Invalid JSON value: {}", e)))?;
        let filter = match op {
            WasmOperator::Equals => Filter::Equals(field, value),
            WasmOperator::GreaterThan => Filter::GreaterThan(field, value),
            WasmOperator::LessThan => Filter::LessThan(field, value),
            WasmOperator::GreaterOrEqual => Filter::GreaterOrEqual(field, value),
            WasmOperator::LessOrEqual => Filter::LessOrEqual(field, value),
            WasmOperator::Contains => {
                if let Value::String(s) = value {
                    Filter::Contains(field, s)
                }
                else {
                    return Ok(());
                }
            },
            WasmOperator::StartsWith => {
                if let Value::String(s) = value {
                    Filter::StartsWith(field, s)
                }
                else {
                    return Ok(());
                }
            },
            WasmOperator::EndsWith => {
                if let Value::String(s) = value {
                    Filter::EndsWith(field, s)
                }
                else {
                    return Ok(());
                }
            },
            WasmOperator::In => {
                if let Value::Array(arr) = value {
                    Filter::In(field, arr)
                }
                else {
                    return Ok(());
                }
            },
            WasmOperator::Exists => Filter::Exists(field, true),
        };
        self.filters.push(filter);
        Ok(())
    }

    pub fn and(&mut self, other: WasmQueryBuilder) -> Result<(), JsValue> {
        if let Some(last) = self.filters.pop() {
            let combined = Filter::And(
                Box::new(last),
                Box::new(
                    other
                        .filters
                        .into_iter()
                        .next()
                        .unwrap_or(Filter::Exists(String::new(), true)),
                ),
            );
            self.filters.push(combined);
        }
        Ok(())
    }

    pub fn or(&mut self, other: WasmQueryBuilder) -> Result<(), JsValue> {
        if let Some(last) = self.filters.pop() {
            let combined = Filter::Or(
                Box::new(last),
                Box::new(
                    other
                        .filters
                        .into_iter()
                        .next()
                        .unwrap_or(Filter::Exists(String::new(), true)),
                ),
            );
            self.filters.push(combined);
        }
        Ok(())
    }

    pub fn sort(&mut self, field: String, order: WasmSortOrder) -> Result<(), JsValue> {
        self.sort = Some((field, order.into()));
        Ok(())
    }

    pub fn limit(&mut self, limit: u32) -> Result<(), JsValue> {
        self.limit = Some(limit as usize);
        Ok(())
    }

    pub fn offset(&mut self, offset: u32) -> Result<(), JsValue> {
        self.offset = Some(offset as usize);
        Ok(())
    }

    pub fn projection(&mut self, fields: Vec<String>) -> Result<(), JsValue> {
        self.projection = Some(fields);
        Ok(())
    }

    pub fn build(&self) -> WasmQuery {
        WasmQuery {
            filters:    self.filters.clone(),
            sort:       self.sort.clone(),
            limit:      self.limit,
            offset:     self.offset,
            projection: self.projection.clone(),
        }
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WasmQuery {
    filters:    Vec<Filter>,
    sort:       Option<(String, SortOrder)>,
    limit:      Option<usize>,
    offset:     Option<usize>,
    projection: Option<Vec<String>>,
}

impl From<WasmQuery> for Query {
    fn from(query: WasmQuery) -> Self {
        Query {
            filters:    query.filters,
            sort:       query.sort,
            limit:      query.limit,
            offset:     query.offset,
            projection: query.projection,
        }
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WasmQueryResult {
    pub documents:         JsValue,
    pub total_count:       u32,
    pub execution_time_ms: f64,
}

#[wasm_bindgen]
pub struct WasmStore {
    store: Store,
}

#[wasm_bindgen]
impl WasmStore {
    #[wasm_bindgen(constructor)]
    pub fn new(path: &str, passphrase: Option<String>) -> Result<WasmStore, JsValue> {
        Err(JsValue::from_str(
            "Use WasmStore.create() instead of new WasmStore()",
        ))
    }

    pub async fn create(path: &str, passphrase: Option<String>) -> Result<WasmStore, JsValue> {
        match Store::new(path, passphrase.as_deref()).await {
            Ok(store) => {
                Ok(WasmStore {
                    store,
                })
            },
            Err(e) => Err(JsValue::from_str(&format!("Failed to create store: {}", e))),
        }
    }

    pub async fn collection(&self, name: &str) -> Result<WasmCollection, JsValue> {
        match self.store.collection(name).await {
            Ok(collection) => {
                Ok(WasmCollection {
                    collection,
                })
            },
            Err(e) => {
                Err(JsValue::from_str(&format!(
                    "Failed to get collection: {}",
                    e
                )))
            },
        }
    }

    pub async fn delete_collection(&self, name: &str) -> Result<(), JsValue> {
        match self.store.delete_collection(name).await {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(JsValue::from_str(&format!(
                    "Failed to delete collection: {}",
                    e
                )))
            },
        }
    }

    pub async fn list_collections(&self) -> Result<JsValue, JsValue> {
        match self.store.list_collections().await {
            Ok(collections) => {
                to_value(&collections).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
            },
            Err(e) => {
                Err(JsValue::from_str(&format!(
                    "Failed to list collections: {}",
                    e
                )))
            },
        }
    }
}

#[wasm_bindgen]
pub struct WasmCollection {
    collection: Collection,
}

#[wasm_bindgen]
impl WasmCollection {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String { self.collection.name().to_string() }

    pub async fn insert(&self, id: &str, data: JsValue) -> Result<(), JsValue> {
        let json_value: Value =
            from_value(data).map_err(|e| JsValue::from_str(&format!("Invalid JSON data: {}", e)))?;
        match self.collection.insert(id, json_value).await {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(JsValue::from_str(&format!(
                    "Failed to insert document: {}",
                    e
                )))
            },
        }
    }

    pub async fn get(&self, id: &str) -> Result<JsValue, JsValue> {
        match self.collection.get(id).await {
            Ok(Some(doc)) => {
                let wasm_doc: WasmDocument = doc.into();
                to_value(&wasm_doc).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
            },
            Ok(None) => Ok(JsValue::NULL),
            Err(e) => Err(JsValue::from_str(&format!("Failed to get document: {}", e))),
        }
    }

    pub async fn get_with_verification(&self, id: &str, options: WasmVerificationOptions) -> Result<JsValue, JsValue> {
        match self
            .collection
            .get_with_verification(id, &options.into())
            .await
        {
            Ok(Some(doc)) => {
                let wasm_doc: WasmDocument = doc.into();
                to_value(&wasm_doc).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
            },
            Ok(None) => Ok(JsValue::NULL),
            Err(e) => Err(JsValue::from_str(&format!("Failed to get document: {}", e))),
        }
    }

    pub async fn delete(&self, id: &str) -> Result<(), JsValue> {
        match self.collection.delete(id).await {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(JsValue::from_str(&format!(
                    "Failed to delete document: {}",
                    e
                )))
            },
        }
    }

    pub async fn count(&self) -> Result<u32, JsValue> {
        match self.collection.count().await {
            Ok(count) => Ok(count as u32),
            Err(e) => {
                Err(JsValue::from_str(&format!(
                    "Failed to count documents: {}",
                    e
                )))
            },
        }
    }

    pub async fn update(&self, id: &str, data: JsValue) -> Result<(), JsValue> {
        let json_value: Value =
            from_value(data).map_err(|e| JsValue::from_str(&format!("Invalid JSON data: {}", e)))?;
        match self.collection.update(id, json_value).await {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(JsValue::from_str(&format!(
                    "Failed to update document: {}",
                    e
                )))
            },
        }
    }

    pub async fn upsert(&self, id: &str, data: JsValue) -> Result<bool, JsValue> {
        let json_value: Value =
            from_value(data).map_err(|e| JsValue::from_str(&format!("Invalid JSON data: {}", e)))?;
        match self.collection.upsert(id, json_value).await {
            Ok(was_insert) => Ok(was_insert),
            Err(e) => {
                Err(JsValue::from_str(&format!(
                    "Failed to upsert document: {}",
                    e
                )))
            },
        }
    }

    pub async fn bulk_insert(&self, documents: JsValue) -> Result<(), JsValue> {
        let docs: Vec<JsBulkDocument> =
            from_value(documents).map_err(|e| JsValue::from_str(&format!("Invalid documents array: {}", e)))?;
        let rust_docs: Vec<(&str, Value)> = docs
            .iter()
            .map(|d| (d.id.as_str(), d.data.clone()))
            .collect();
        match self.collection.bulk_insert(rust_docs).await {
            Ok(_) => Ok(()),
            Err(e) => Err(JsValue::from_str(&format!("Failed to bulk insert: {}", e))),
        }
    }

    pub async fn list(&self) -> Result<JsValue, JsValue> {
        let ids: Result<Vec<String>, _> = self.collection.list().try_collect().await;
        match ids {
            Ok(ids) => to_value(&ids).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
            Err(e) => {
                Err(JsValue::from_str(&format!(
                    "Failed to list documents: {}",
                    e
                )))
            },
        }
    }

    pub async fn all(&self) -> Result<JsValue, JsValue> {
        let docs: Result<Vec<Document>, _> = self.collection.all().try_collect().await;
        match docs {
            Ok(docs) => {
                let wasm_docs: Vec<WasmDocument> = docs.into_iter().map(|d| d.into()).collect();
                to_value(&wasm_docs).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
            },
            Err(e) => {
                Err(JsValue::from_str(&format!(
                    "Failed to get all documents: {}",
                    e
                )))
            },
        }
    }

    pub async fn all_with_verification(&self, options: WasmVerificationOptions) -> Result<JsValue, JsValue> {
        let docs: Result<Vec<Document>, _> = self
            .collection
            .all_with_verification(&options.into())
            .try_collect()
            .await;
        match docs {
            Ok(docs) => {
                let wasm_docs: Vec<WasmDocument> = docs.into_iter().map(|d| d.into()).collect();
                to_value(&wasm_docs).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
            },
            Err(e) => {
                Err(JsValue::from_str(&format!(
                    "Failed to get all documents: {}",
                    e
                )))
            },
        }
    }

    pub async fn query(&self, query: WasmQuery) -> Result<WasmQueryResult, JsValue> {
        self.query_with_verification(query, WasmVerificationOptions::default())
            .await
    }

    pub async fn query_with_verification(
        &self,
        query: WasmQuery,
        options: WasmVerificationOptions,
    ) -> Result<WasmQueryResult, JsValue> {
        match self
            .collection
            .query_with_verification(query.into(), &options.into())
            .await
        {
            Ok(result) => {
                let docs: Result<Vec<Document>, _> = result.documents.try_collect().await;
                match docs {
                    Ok(documents) => {
                        let wasm_docs: Vec<WasmDocument> = documents.into_iter().map(|d| d.into()).collect();
                        let js_docs = to_value(&wasm_docs)
                            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
                        Ok(WasmQueryResult {
                            documents:         js_docs,
                            total_count:       result.total_count.unwrap_or(0) as u32,
                            execution_time_ms: result.execution_time.as_secs_f64() * 1000.0,
                        })
                    },
                    Err(e) => {
                        Err(JsValue::from_str(&format!(
                            "Failed to collect query results: {}",
                            e
                        )))
                    },
                }
            },
            Err(e) => {
                Err(JsValue::from_str(&format!(
                    "Failed to execute query: {}",
                    e
                )))
            },
        }
    }
}

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct JsBulkDocument {
    pub id:   String,
    pub data: JsValue,
}

#[wasm_bindgen]
#[derive(Debug, Clone, serde::Serialize)]
pub struct WasmDocument {
    pub id:         String,
    pub version:    u32,
    pub created_at: String,
    pub updated_at: String,
    pub hash:       String,
    pub signature:  String,
    pub data:       JsValue,
}

impl From<Document> for WasmDocument {
    fn from(doc: Document) -> Self {
        let js_data = to_value(&doc.data()).unwrap_or(JsValue::NULL);
        Self {
            id:         doc.id().to_string(),
            version:    doc.version(),
            created_at: doc.created_at().to_rfc3339(),
            updated_at: doc.updated_at().to_rfc3339(),
            hash:       doc.hash().to_string(),
            signature:  doc.signature().to_string(),
            data:       js_data,
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String { self.id.clone() }

    #[wasm_bindgen(getter)]
    pub fn version(&self) -> u32 { self.version }

    #[wasm_bindgen(getter)]
    pub fn created_at(&self) -> String { self.created_at.clone() }

    #[wasm_bindgen(getter)]
    pub fn updated_at(&self) -> String { self.updated_at.clone() }

    #[wasm_bindgen(getter)]
    pub fn hash(&self) -> String { self.hash.clone() }

    #[wasm_bindgen(getter)]
    pub fn signature(&self) -> String { self.signature.clone() }

    #[wasm_bindgen(getter)]
    pub fn data(&self) -> JsValue { self.data.clone() }
}

#[wasm_bindgen(start)]
pub fn main() {
    console_log!("Cyberpath Sentinel WebAssembly module initialized");
}
