use std::time::Duration;

use napi::bindgen_prelude::*;
use napi_derive::napi;
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
use serde_json::Value;

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

impl From<JsOperator> for Operator {
    fn from(op: JsOperator) -> Self {
        match op {
            JsOperator::Equals => Operator::Equals,
            JsOperator::GreaterThan => Operator::GreaterThan,
            JsOperator::LessThan => Operator::LessThan,
            JsOperator::GreaterOrEqual => Operator::GreaterOrEqual,
            JsOperator::LessOrEqual => Operator::LessOrEqual,
            JsOperator::Contains => Operator::Contains,
            JsOperator::StartsWith => Operator::StartsWith,
            JsOperator::EndsWith => Operator::EndsWith,
            JsOperator::In => Operator::In,
            JsOperator::Exists => Operator::Exists,
        }
    }
}

impl From<JsSortOrder> for SortOrder {
    fn from(order: JsSortOrder) -> Self {
        match order {
            JsSortOrder::Ascending => SortOrder::Ascending,
            JsSortOrder::Descending => SortOrder::Descending,
        }
    }
}

impl From<JsVerificationMode> for VerificationMode {
    fn from(mode: JsVerificationMode) -> Self {
        match mode {
            JsVerificationMode::Strict => VerificationMode::Strict,
            JsVerificationMode::Warn => VerificationMode::Warn,
            JsVerificationMode::Silent => VerificationMode::Silent,
        }
    }
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
    pub fn new() -> Self { Self::default() }

    #[napi]
    pub fn strict() -> Self {
        let opts = VerificationOptions::strict();
        Self::from_opts(opts)
    }

    #[napi]
    pub fn disabled() -> Self {
        let opts = VerificationOptions::disabled();
        Self::from_opts(opts)
    }

    #[napi]
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

impl Default for JsVerificationOptions {
    fn default() -> Self {
        let opts = VerificationOptions::default();
        Self::from_opts(opts)
    }
}

impl From<JsVerificationOptions> for VerificationOptions {
    fn from(opts: JsVerificationOptions) -> Self {
        Self {
            verify_signature:            opts.verify_signature,
            verify_hash:                 opts.verify_hash,
            signature_verification_mode: opts.signature_verification_mode.into(),
            empty_signature_mode:        opts.empty_signature_mode.into(),
            hash_verification_mode:      opts.hash_verification_mode.into(),
        }
    }
}

#[napi]
pub struct JsQueryBuilder {
    filters:    Vec<Filter>,
    sort:       Option<(String, SortOrder)>,
    limit:      Option<usize>,
    offset:     Option<usize>,
    projection: Option<Vec<String>>,
}

#[napi]
impl JsQueryBuilder {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            filters:    Vec::new(),
            sort:       None,
            limit:      None,
            offset:     None,
            projection: None,
        }
    }

    #[napi]
    pub fn filter(&mut self, field: String, op: JsOperator, value: Value) -> &mut JsQueryBuilder {
        let filter = match op {
            JsOperator::Equals => Filter::Equals(field, value),
            JsOperator::GreaterThan => Filter::GreaterThan(field, value),
            JsOperator::LessThan => Filter::LessThan(field, value),
            JsOperator::GreaterOrEqual => Filter::GreaterOrEqual(field, value),
            JsOperator::LessOrEqual => Filter::LessOrEqual(field, value),
            JsOperator::Contains => {
                if let Value::String(s) = value {
                    Filter::Contains(field, s)
                }
                else {
                    return self;
                }
            },
            JsOperator::StartsWith => {
                if let Value::String(s) = value {
                    Filter::StartsWith(field, s)
                }
                else {
                    return self;
                }
            },
            JsOperator::EndsWith => {
                if let Value::String(s) = value {
                    Filter::EndsWith(field, s)
                }
                else {
                    return self;
                }
            },
            JsOperator::In => {
                if let Value::Array(arr) = value {
                    Filter::In(field, arr)
                }
                else {
                    return self;
                }
            },
            JsOperator::Exists => Filter::Exists(field, true),
        };
        self.filters.push(filter);
        self
    }

    #[napi]
    pub fn and(&mut self, other: JsQueryBuilder) -> &mut JsQueryBuilder {
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
        self
    }

    #[napi]
    pub fn or(&mut self, other: JsQueryBuilder) -> &mut JsQueryBuilder {
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
        self
    }

    #[napi]
    pub fn sort(&mut self, field: String, order: JsSortOrder) -> &mut JsQueryBuilder {
        self.sort = Some((field, order.into()));
        self
    }

    #[napi]
    pub fn limit(&mut self, limit: u32) -> &mut JsQueryBuilder {
        self.limit = Some(limit as usize);
        self
    }

    #[napi]
    pub fn offset(&mut self, offset: u32) -> &mut JsQueryBuilder {
        self.offset = Some(offset as usize);
        self
    }

    #[napi]
    pub fn projection(&mut self, fields: Vec<String>) -> &mut JsQueryBuilder {
        self.projection = Some(fields);
        self
    }

    #[napi]
    pub fn build(&self) -> JsQuery {
        JsQuery {
            filters:    self.filters.clone(),
            sort:       self.sort.clone(),
            limit:      self.limit,
            offset:     self.offset,
            projection: self.projection.clone(),
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
pub struct JsFilter {
    pub field:       String,
    pub filter_type: JsFilterType,
    pub value:       Value,
    pub children:    Option<Vec<JsFilter>>,
}

impl From<JsQuery> for Query {
    fn from(query: JsQuery) -> Self {
        let filters: Vec<Filter> = query.filters.into_iter().map(|f| f.into()).collect();
        Query {
            filters,
            sort: query.sort.map(|s| (s.field, s.order.into())),
            limit: query.limit.map(|l| l as usize),
            offset: query.offset.map(|o| o as usize),
            projection: query.projection,
        }
    }
}

impl From<JsFilter> for Filter {
    fn from(js_filter: JsFilter) -> Self {
        match js_filter.filter_type {
            JsFilterType::Equals => Filter::Equals(js_filter.field, js_filter.value),
            JsFilterType::GreaterThan => Filter::GreaterThan(js_filter.field, js_filter.value),
            JsFilterType::LessThan => Filter::LessThan(js_filter.field, js_filter.value),
            JsFilterType::GreaterOrEqual => Filter::GreaterOrEqual(js_filter.field, js_filter.value),
            JsFilterType::LessOrEqual => Filter::LessOrEqual(js_filter.field, js_filter.value),
            JsFilterType::Contains => {
                if let Value::String(s) = js_filter.value {
                    Filter::Contains(js_filter.field, s)
                }
                else {
                    Filter::Exists(js_filter.field, true)
                }
            },
            JsFilterType::StartsWith => {
                if let Value::String(s) = js_filter.value {
                    Filter::StartsWith(js_filter.field, s)
                }
                else {
                    Filter::Exists(js_filter.field, true)
                }
            },
            JsFilterType::EndsWith => {
                if let Value::String(s) = js_filter.value {
                    Filter::EndsWith(js_filter.field, s)
                }
                else {
                    Filter::Exists(js_filter.field, true)
                }
            },
            JsFilterType::In => {
                if let Value::Array(arr) = js_filter.value {
                    Filter::In(js_filter.field, arr)
                }
                else {
                    Filter::Exists(js_filter.field, true)
                }
            },
            JsFilterType::Exists => {
                if let Value::Bool(b) = js_filter.value {
                    Filter::Exists(js_filter.field, b)
                }
                else {
                    Filter::Exists(js_filter.field, true)
                }
            },
            JsFilterType::And => {
                if let Some(children) = js_filter.children {
                    if children.len() >= 2 {
                        let left = Box::new(children[0].clone().into());
                        let right = Box::new(children[1].clone().into());
                        Filter::And(left, right)
                    }
                    else {
                        Filter::Exists(js_filter.field, true)
                    }
                }
                else {
                    Filter::Exists(js_filter.field, true)
                }
            },
            JsFilterType::Or => {
                if let Some(children) = js_filter.children {
                    if children.len() >= 2 {
                        let left = Box::new(children[0].clone().into());
                        let right = Box::new(children[1].clone().into());
                        Filter::Or(left, right)
                    }
                    else {
                        Filter::Exists(js_filter.field, true)
                    }
                }
                else {
                    Filter::Exists(js_filter.field, true)
                }
            },
        }
    }
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
    pub fn constructor(path: String, passphrase: Option<String>) -> napi::Result<Self> {
        Err(napi::Error::from_reason(
            "Use Store.create() instead of new Store()",
        ))
    }

    #[napi]
    pub async fn create(path: String, passphrase: Option<String>) -> napi::Result<Self> {
        let passphrase = passphrase.as_deref();
        match Store::new(&path, passphrase).await {
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
    pub async fn get_with_verification(
        &self,
        id: String,
        options: JsVerificationOptions,
    ) -> napi::Result<Option<JsDocument>> {
        match self
            .collection
            .get_with_verification(&id, &options.into())
            .await
        {
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
    pub async fn get_many(&self, ids: Vec<String>) -> napi::Result<Vec<Option<JsDocument>>> {
        let ids_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
        match self.collection.get_many(&ids_refs).await {
            Ok(docs) => {
                let result: Vec<Option<JsDocument>> = docs
                    .into_iter()
                    .map(|opt| opt.map(|d| JsDocument::from(d)))
                    .collect();
                Ok(result)
            },
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to get many documents: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn aggregate(
        &self,
        filters: Vec<JsFilter>,
        aggregation_type: String,
        field: Option<String>,
    ) -> napi::Result<Value> {
        let filters_converted: Vec<Filter> = filters.into_iter().map(|f| f.into()).collect();
        let aggregation = match aggregation_type.as_str() {
            "Count" => sentinel_dbms::Aggregation::Count,
            "Sum" => sentinel_dbms::Aggregation::Sum(field.unwrap_or_default()),
            "Avg" => sentinel_dbms::Aggregation::Avg(field.unwrap_or_default()),
            "Min" => sentinel_dbms::Aggregation::Min(field.unwrap_or_default()),
            "Max" => sentinel_dbms::Aggregation::Max(field.unwrap_or_default()),
            _ => {
                return Err(napi::Error::from_reason(format!(
                    "Unknown aggregation type: {}",
                    aggregation_type
                )))
            },
        };
        match self
            .collection
            .aggregate(filters_converted, aggregation)
            .await
        {
            Ok(result) => Ok(result),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to aggregate: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn list(&self) -> napi::Result<Vec<String>> {
        let ids: Result<Vec<String>, _> = self.collection.list().try_collect().await;
        match ids {
            Ok(ids) => Ok(ids),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to list documents: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn all(&self) -> napi::Result<Vec<JsDocument>> {
        let docs: Result<Vec<Document>, _> = self.collection.all().try_collect().await;
        match docs {
            Ok(docs) => Ok(docs.into_iter().map(|d| d.into()).collect()),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to get all documents: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn all_with_verification(&self, options: JsVerificationOptions) -> napi::Result<Vec<JsDocument>> {
        let docs: Result<Vec<Document>, _> = self
            .collection
            .all_with_verification(&options.into())
            .try_collect()
            .await;
        match docs {
            Ok(docs) => Ok(docs.into_iter().map(|d| d.into()).collect()),
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to get all documents: {}",
                    e
                )))
            },
        }
    }

    #[napi]
    pub async fn query(&self, query: JsQuery) -> napi::Result<JsQueryResult> {
        self.query_with_verification(query, JsVerificationOptions::default())
            .await
    }

    #[napi]
    pub async fn query_with_verification(
        &self,
        query: JsQuery,
        options: JsVerificationOptions,
    ) -> napi::Result<JsQueryResult> {
        match self
            .collection
            .query_with_verification(query.into(), &options.into())
            .await
        {
            Ok(result) => {
                let docs: Result<Vec<Document>, _> = result.documents.try_collect().await;
                match docs {
                    Ok(documents) => {
                        Ok(JsQueryResult {
                            documents:         documents.into_iter().map(|d| d.into()).collect(),
                            total_count:       result.total_count.unwrap_or(0) as u32,
                            execution_time_ms: result.execution_time.as_secs_f64() * 1000.0,
                        })
                    },
                    Err(e) => {
                        Err(napi::Error::from_reason(format!(
                            "Failed to collect query results: {}",
                            e
                        )))
                    },
                }
            },
            Err(e) => {
                Err(napi::Error::from_reason(format!(
                    "Failed to execute query: {}",
                    e
                )))
            },
        }
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

#[napi]
pub struct JsDocumentRef {
    document: Document,
}

#[napi]
impl JsDocumentRef {
    #[napi(getter)]
    pub fn id(&self) -> String { self.document.id().to_string() }

    #[napi(getter)]
    pub fn version(&self) -> u32 { self.document.version() }

    #[napi(getter)]
    pub fn created_at(&self) -> String { self.document.created_at().to_rfc3339() }

    #[napi(getter)]
    pub fn updated_at(&self) -> String { self.document.updated_at().to_rfc3339() }

    #[napi(getter)]
    pub fn hash(&self) -> String { self.document.hash().to_string() }

    #[napi(getter)]
    pub fn signature(&self) -> String { self.document.signature().to_string() }

    #[napi(getter)]
    pub fn data(&self) -> Value { self.document.data().clone() }
}

impl From<Document> for JsDocumentRef {
    fn from(document: Document) -> Self {
        Self {
            document,
        }
    }
}
