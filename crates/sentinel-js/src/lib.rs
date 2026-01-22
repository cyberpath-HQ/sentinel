use napi_derive::napi;
use sentinel_dbms::{Collection, Document, Store};
use serde_json::Value;
use futures::StreamExt;

#[napi]
pub enum JsOperator {
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

#[napi]
pub enum JsSortOrder {
    Ascending,
    Descending,
}

#[napi]
pub enum JsVerificationMode {
    Strict,
    Warn,
    Silent,
}

#[napi]
pub struct JsVerificationOptions {
    pub verify_signature:            bool,
    pub verify_hash:                 bool,
    pub signature_verification_mode: JsVerificationMode,
    pub empty_signature_mode:        JsVerificationMode,
    pub hash_verification_mode:      JsVerificationMode,
}

#[napi]
impl JsVerificationOptions {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: JsVerificationMode::Strict,
            empty_signature_mode:        JsVerificationMode::Warn,
            hash_verification_mode:      JsVerificationMode::Strict,
        }
    }

    #[napi]
    pub fn strict() -> Self {
        Self {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: JsVerificationMode::Strict,
            empty_signature_mode:        JsVerificationMode::Strict,
            hash_verification_mode:      JsVerificationMode::Strict,
        }
    }

    #[napi]
    pub fn disabled() -> Self {
        Self {
            verify_signature:            false,
            verify_hash:                 false,
            signature_verification_mode: JsVerificationMode::Silent,
            empty_signature_mode:        JsVerificationMode::Silent,
            hash_verification_mode:      JsVerificationMode::Silent,
        }
    }

    #[napi]
    pub fn warn() -> Self {
        Self {
            verify_signature:            true,
            verify_hash:                 true,
            signature_verification_mode: JsVerificationMode::Warn,
            empty_signature_mode:        JsVerificationMode::Warn,
            hash_verification_mode:      JsVerificationMode::Warn,
        }
    }
}

#[napi(object)]
pub struct JsQuery {
    pub filters:    Vec<JsFilter>,
    pub sort:       Option<JsSort>,
    pub limit:      Option<u32>,
    pub offset:     Option<u32>,
    pub projection: Option<Vec<String>>,
}

#[napi(object)]
pub struct JsSort {
    pub field: String,
    pub order: JsSortOrder,
}

#[napi]
pub enum JsFilterType {
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
    And,
    Or,
}

#[napi(object)]
#[derive(Clone)]
pub struct JsFilter {
    pub field:       String,
    pub filter_type: JsFilterType,
    pub value:       Value,
    pub children:    Option<Vec<JsFilter>>,
}

#[napi(object)]
pub struct JsQueryResult {
    pub documents:         Vec<JsDocument>,
    pub total_count:       u32,
    pub execution_time_ms: f64,
}

#[napi]
pub struct JsStore {
    store: Store,
}

#[napi]
impl JsStore {
    #[napi(constructor)]
    pub fn constructor(_path: String, _passphrase: Option<String>) -> napi::Result<Self> {
        Err(napi::Error::from_reason(
            "Use Store.create() instead of new Store()",
        ))
    }

    #[napi]
    pub async fn create(path: String, passphrase: Option<String>) -> napi::Result<Self> {
        match Store::new(&path, passphrase.as_deref()).await {
            Ok(store) => {
                Ok(JsStore {
                    store,
                })
            },
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to create store: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn collection(&self, name: String) -> napi::Result<JsCollection> {
        match self.store.collection(&name).await {
            Ok(collection) => {
                Ok(JsCollection {
                    collection,
                })
            },
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to get collection: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn delete_collection(&self, name: String) -> napi::Result<()> {
        match self.store.delete_collection(&name).await {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to delete collection: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn list_collections(&self) -> napi::Result<Vec<String>> {
        match self.store.list_collections().await {
            Ok(collections) => Ok(collections),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to list collections: {}",
                    e
                )))
            },
        }
    }
}

#[napi]
pub struct JsCollection {
    collection: Collection,
}

#[napi]
impl JsCollection {
    #[napi(getter)]
    pub fn name(&self) -> String { self.collection.name().to_string() }

    #[napi]
    pub async fn insert(&self, id: String, data: Value) -> napi::Result<()> {
        match self.collection.insert(&id, data).await {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to insert document: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn get(&self, id: String) -> napi::Result<Option<JsDocument>> {
        match self.collection.get(&id).await {
            Ok(Some(doc)) => Ok(Some(doc.into())),
            Ok(None) => Ok(None),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to get document: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn delete(&self, id: String) -> napi::Result<()> {
        match self.collection.delete(&id).await {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to delete document: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn count(&self) -> napi::Result<u32> {
        match self.collection.count().await {
            Ok(count) => Ok(count as u32),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to count documents: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn update(&self, id: String, data: Value) -> napi::Result<()> {
        match self.collection.update(&id, data).await {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to update document: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn upsert(&self, id: String, data: Value) -> napi::Result<bool> {
        match self.collection.upsert(&id, data).await {
            Ok(was_insert) => Ok(was_insert),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to upsert document: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn bulk_insert(&self, documents: Vec<JsBulkDocument>) -> napi::Result<()> {
        let docs: Vec<(&str, Value)> = documents
            .iter()
            .map(|d| (d.id.as_str(), d.data.clone()))
            .collect();
        match self.collection.bulk_insert(docs).await {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to bulk insert: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn list(&self) -> napi::Result<Vec<String>> {
        let mut ids = Vec::new();
        let mut stream = self.collection.list();
        while let Some(result) = stream.next().await {
            match result {
                Ok(id) => ids.push(id),
                Err(e) => {
                    return Err(napi::Error::from_reason(format!(
                        "Failed to list documents: {}",
                        e
                    )))
                },
            }
        }
        Ok(ids)
    }

    #[napi]
    pub async fn all(&self) -> napi::Result<Vec<JsDocument>> {
        let mut docs: Vec<JsDocument> = Vec::new();
        let mut stream = self.collection.all();
        while let Some(result) = stream.next().await {
            match result {
                Ok(doc) => docs.push(doc.into()),
                Err(e) => {
                    return Err(napi::Error::from_reason(format!(
                        "Failed to get all documents: {}",
                        e
                    )))
                },
            }
        }
        Ok(docs)
    }
}

#[napi(object)]
pub struct JsBulkDocument {
    pub id:   String,
    pub data: Value,
}

#[napi(object)]
pub struct JsDocument {
    pub id:         String,
    pub version:    u32,
    pub created_at: String,
    pub updated_at: String,
    pub hash:       String,
    pub signature:  String,
    pub data:       Value,
}

impl From<Document> for JsDocument {
    fn from(doc: Document) -> Self {
        Self {
            id:         doc.id().to_string(),
            version:    doc.version(),
            created_at: doc.created_at().to_rfc3339(),
            updated_at: doc.updated_at().to_rfc3339(),
            hash:       doc.hash().to_string(),
            signature:  doc.signature().to_string(),
            data:       doc.data().clone(),
        }
    }
}
