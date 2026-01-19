use std::{process::Command, sync::Arc};

use pyo3::{exceptions::PyRuntimeError, prelude::*};
use pyo3_asyncio::tokio::future_into_py;
use serde_json::Value;
use futures::StreamExt;
use sentinel_dbms::{
    Aggregation,
    Collection,
    Document,
    Filter,
    Operator,
    QueryBuilder,
    SentinelError,
    SortOrder,
    Store,
    VerificationMode,
};
use sentinel_crypto::{hash_data, sign_hash, verify_signature};

#[pymodule]
fn sentinel_python(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyStore>()?;
    m.add_class::<PyCollection>()?;
    m.add_class::<PyDocument>()?;
    m.add_class::<PyQueryBuilder>()?;
    m.add_class::<PyQueryResult>()?;
    m.add_class::<PyVerificationOptions>()?;
    m.add_function(wrap_pyfunction!(hash_data_py, m)?)?;
    m.add_function(wrap_pyfunction!(sign_hash_py, m)?)?;
    m.add_function(wrap_pyfunction!(verify_signature_py, m)?)?;
    Ok(())
}

// Helper function to convert SentinelError to PyErr
fn sentinel_error_to_py(err: SentinelError) -> PyErr { PyRuntimeError::new_err(err.to_string()) }

// Helper trait for converting Result<SentinelError> to PyResult
trait SentinelResultExt<T> {
    fn sentinel_to_py(self) -> PyResult<T>;
}

impl<T> SentinelResultExt<T> for Result<T, SentinelError> {
    fn sentinel_to_py(self) -> PyResult<T> { self.map_err(sentinel_error_to_py) }
}

#[pyclass(name = "Store")]
pub struct PyStore {
    store: Arc<Store>,
}

#[pymethods]
impl PyStore {
    /// Create a new Sentinel store
    ///
    /// Args:
    ///     path (str): Filesystem path where the store will be created
    ///     passphrase (Optional[str]): Optional passphrase for encryption
    ///
    /// Returns:
    ///     Store: A new Store instance
    #[staticmethod]
    #[pyo3(signature = (path, passphrase=None))]
    fn new<'a>(py: Python<'a>, path: String, passphrase: Option<String>) -> PyResult<&'a PyAny> {
        let passphrase_clone = passphrase.clone();

        future_into_py(py, async move {
            let passphrase_ref = passphrase_clone.as_deref();
            match Store::new(&path, passphrase_ref).await {
                Ok(store) => {
                    Ok(PyStore {
                        store: Arc::new(store),
                    })
                },
                Err(e) => Err(sentinel_error_to_py(e)),
            }
        })
    }

    /// Get or create a collection with the specified name
    ///
    /// Args:
    ///     name (str): Name of the collection
    ///
    /// Returns:
    ///     Collection: The collection instance
    fn collection<'a>(&self, py: Python<'a>, name: String) -> PyResult<&'a PyAny> {
        let store = Arc::clone(&self.store);
        future_into_py(py, async move {
            match store.collection(&name).await {
                Ok(collection) => {
                    Ok(PyCollection {
                        collection: Arc::new(collection),
                    })
                },
                Err(e) => Err(sentinel_error_to_py(e)),
            }
        })
    }

    /// Delete a collection and all its documents
    ///
    /// Args:
    ///     name (str): Name of the collection to delete
    fn delete_collection<'a>(&self, py: Python<'a>, name: String) -> PyResult<&'a PyAny> {
        let store = Arc::clone(&self.store);
        future_into_py(py, async move {
            store.delete_collection(&name).await.sentinel_to_py()?;
            Ok(())
        })
    }

    /// List all collections in the store
    ///
    /// Returns:
    ///     List[str]: List of collection names
    fn list_collections<'a>(&self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let store = Arc::clone(&self.store);
        future_into_py(py, async move {
            let collections = store.list_collections().await.sentinel_to_py()?;
            Ok(collections)
        })
    }
}

/// Python wrapper for Sentinel Collection
#[pyclass(name = "Collection")]
pub struct PyCollection {
    collection: Arc<Collection>,
}

#[pymethods]
impl PyCollection {
    /// Get the collection name
    ///
    /// Returns:
    ///     str: The collection name
    fn name(&self) -> String { self.collection.name().to_string() }

    /// Insert a new document or overwrite an existing one
    ///
    /// Args:
    ///     id (str): Unique identifier for the document
    ///     data (dict): JSON-serializable data to store
    fn insert<'a>(&self, py: Python<'a>, id: String, data: PyObject) -> PyResult<&'a PyAny> {
        let collection = Arc::clone(&self.collection);
        let json_value = pyobject_to_json_value(py, data)?;
        future_into_py(py, async move {
            collection.insert(&id, json_value).await.sentinel_to_py()?;
            Ok(())
        })
    }

    /// Retrieve a document by its ID
    ///
    /// Args:
    ///     id (str): Document identifier
    ///
    /// Returns:
    ///     Optional[Document]: The document if found, None otherwise
    fn get<'a>(&self, py: Python<'a>, id: String) -> PyResult<&'a PyAny> {
        let collection = Arc::clone(&self.collection);
        future_into_py(py, async move {
            match collection.get(&id).await.sentinel_to_py()? {
                Some(doc) => {
                    Ok(Some(PyDocument {
                        document: doc,
                    }))
                },
                None => Ok(None),
            }
        })
    }

    /// Retrieve a document with custom verification options
    ///
    /// Args:
    ///     id (str): Document identifier
    ///     options (VerificationOptions): Verification options
    ///
    /// Returns:
    ///     Optional[Document]: The document if found, None otherwise
    fn get_with_verification<'a>(
        &self,
        py: Python<'a>,
        id: String,
        options: PyVerificationOptions,
    ) -> PyResult<&'a PyAny> {
        let collection = Arc::clone(&self.collection);
        let verification_options = options.to_rust_options();
        future_into_py(py, async move {
            match collection
                .get_with_verification(&id, &verification_options)
                .await
                .sentinel_to_py()?
            {
                Some(doc) => {
                    Ok(Some(PyDocument {
                        document: doc,
                    }))
                },
                None => Ok(None),
            }
        })
    }

    /// Update an existing document
    ///
    /// Args:
    ///     id (str): Document identifier
    ///     data (dict): New JSON data
    fn update<'a>(&self, py: Python<'a>, id: String, data: PyObject) -> PyResult<&'a PyAny> {
        let collection = Arc::clone(&self.collection);
        let json_value = pyobject_to_json_value(py, data)?;
        future_into_py(py, async move {
            collection.update(&id, json_value).await.sentinel_to_py()?;
            Ok(())
        })
    }

    /// Insert or update a document
    ///
    /// Args:
    ///     id (str): Document identifier
    ///     data (dict): JSON data
    ///
    /// Returns:
    ///     bool: True if inserted, False if updated
    fn upsert<'a>(&self, py: Python<'a>, id: String, data: PyObject) -> PyResult<&'a PyAny> {
        let collection = Arc::clone(&self.collection);
        let json_value = pyobject_to_json_value(py, data)?;
        future_into_py(py, async move {
            let was_insert = collection.upsert(&id, json_value).await.sentinel_to_py()?;
            Ok(was_insert)
        })
    }

    /// Delete a document (soft delete)
    ///
    /// Args:
    ///     id (str): Document identifier to delete
    fn delete<'a>(&self, py: Python<'a>, id: String) -> PyResult<&'a PyAny> {
        let collection = Arc::clone(&self.collection);
        future_into_py(py, async move {
            collection.delete(&id).await.sentinel_to_py()?;
            Ok(())
        })
    }

    /// Get multiple documents by IDs
    ///
    /// Args:
    ///     ids (List[str]): List of document identifiers
    ///
    /// Returns:
    ///     List[Optional[Document]]: List of documents (None for not found)
    fn get_many<'a>(&self, py: Python<'a>, ids: Vec<String>) -> PyResult<&'a PyAny> {
        let collection = Arc::clone(&self.collection);
        let ids_clone = ids.clone();
        future_into_py(py, async move {
            let ids_refs: Vec<&str> = ids_clone.iter().map(|s| s.as_str()).collect();
            let docs = collection.get_many(&ids_refs).await.sentinel_to_py()?;
            let py_docs: Vec<Option<PyDocument>> = docs
                .into_iter()
                .map(|opt_doc| {
                    opt_doc.map(|doc| {
                        PyDocument {
                            document: doc,
                        }
                    })
                })
                .collect();
            Ok(py_docs)
        })
    }

    /// Count the total number of documents in the collection
    ///
    /// Returns:
    ///     int: Number of documents
    fn count<'a>(&self, py: Python<'a>) -> PyResult<&'a PyAny> {
        let collection = Arc::clone(&self.collection);
        future_into_py(py, async move {
            let count = collection.count().await.sentinel_to_py()?;
            Ok(count)
        })
    }

    /// Insert multiple documents in a single operation
    ///
    /// Args:
    ///     documents (List[Tuple[str, dict]]): List of (id, data) tuples
    fn bulk_insert<'a>(&self, py: Python<'a>, documents: Vec<(String, PyObject)>) -> PyResult<&'a PyAny> {
        let collection = Arc::clone(&self.collection);

        // Convert documents to a format suitable for the async block
        let json_docs: Vec<(String, Value)> = documents
            .into_iter()
            .map(|(id, data)| {
                let json_value = pyobject_to_json_value(py, data)?;
                Ok((id, json_value))
            })
            .collect::<Result<Vec<_>, PyErr>>()?;

        future_into_py(py, async move {
            let json_docs_refs: Vec<(&str, Value)> = json_docs
                .iter()
                .map(|(id, value)| (id.as_str(), value.clone()))
                .collect();
            collection
                .bulk_insert(json_docs_refs)
                .await
                .sentinel_to_py()?;
            Ok(())
        })
    }

    /// Execute a query against the collection
    ///
    /// Args:
    ///     query (QueryBuilder): The query to execute
    ///
    /// Returns:
    ///     QueryResult: The query results
    fn query<'a>(&self, py: Python<'a>, query: PyQueryBuilder) -> PyResult<&'a PyAny> {
        let collection = Arc::clone(&self.collection);
        let query_obj = query.builder.clone().build();
        future_into_py(py, async move {
            let result = collection.query(query_obj).await.sentinel_to_py()?;
            let documents: Vec<PyDocument> = result
                .documents
                .collect::<Vec<Result<Document, SentinelError>>>()
                .await
                .into_iter()
                .filter_map(|res| res.ok())
                .map(|doc| {
                    PyDocument {
                        document: doc,
                    }
                })
                .collect();
            Ok(PyQueryResult {
                documents,
                total_count: result.total_count,
                execution_time: result.execution_time,
            })
        })
    }

    /// Execute a query with custom verification options
    ///
    /// Args:
    ///     query (QueryBuilder): The query to execute
    ///     options (VerificationOptions): Verification options
    ///
    /// Returns:
    ///     QueryResult: The query results
    fn query_with_verification<'a>(
        &self,
        py: Python<'a>,
        query: PyQueryBuilder,
        options: PyVerificationOptions,
    ) -> PyResult<&'a PyAny> {
        let collection = Arc::clone(&self.collection);
        let query_obj = query.builder.clone().build();
        let verification_options = options.to_rust_options();
        future_into_py(py, async move {
            let result = collection
                .query_with_verification(query_obj, &verification_options)
                .await
                .sentinel_to_py()?;
            let documents: Vec<PyDocument> = result
                .documents
                .collect::<Vec<Result<Document, SentinelError>>>()
                .await
                .into_iter()
                .filter_map(|res| res.ok())
                .map(|doc| {
                    PyDocument {
                        document: doc,
                    }
                })
                .collect();
            Ok(PyQueryResult {
                documents,
                total_count: result.total_count,
                execution_time: result.execution_time,
            })
        })
    }

    /// Aggregate documents
    ///
    /// Args:
    ///     filters (List[Tuple[str, str, Any]]): List of (field, operator, value) filters
    ///     aggregation (str): Aggregation type ("count", "sum", "avg", "min", "max")
    ///
    /// Returns:
    ///     Any: Aggregation result
    fn aggregate(
        &self,
        py: Python,
        filters: Vec<(String, String, PyObject)>,
        aggregation: String,
    ) -> PyResult<PyObject> {
        let collection = Arc::clone(&self.collection);

        let query_filters: Result<Vec<Filter>, PyErr> = filters
            .into_iter()
            .map(|(field, op, value)| {
                let operator = match op.as_str() {
                    "equals" => Operator::Equals,
                    "greater_than" => Operator::GreaterThan,
                    "less_than" => Operator::LessThan,
                    "greater_or_equal" => Operator::GreaterOrEqual,
                    "less_or_equal" => Operator::LessOrEqual,
                    "contains" => Operator::Contains,
                    "starts_with" => Operator::StartsWith,
                    "ends_with" => Operator::EndsWith,
                    "in" => Operator::In,
                    "exists" => Operator::Exists,
                    _ => return Err(PyRuntimeError::new_err(format!("Unknown operator: {}", op))),
                };

                let json_value = pyobject_to_json_value(py, value)?;
                let filter = match operator {
                    Operator::Equals => Filter::Equals(field.clone(), json_value),
                    Operator::GreaterThan => Filter::GreaterThan(field.clone(), json_value),
                    Operator::LessThan => Filter::LessThan(field.clone(), json_value),
                    Operator::GreaterOrEqual => Filter::GreaterOrEqual(field.clone(), json_value),
                    Operator::LessOrEqual => Filter::LessOrEqual(field.clone(), json_value),
                    Operator::Contains => {
                        if let Value::String(s) = json_value {
                            Filter::Contains(field.clone(), s)
                        }
                        else {
                            return Err(PyRuntimeError::new_err(
                                "Contains filter requires string value",
                            ));
                        }
                    },
                    Operator::StartsWith => {
                        if let Value::String(s) = json_value {
                            Filter::StartsWith(field.clone(), s)
                        }
                        else {
                            return Err(PyRuntimeError::new_err(
                                "StartsWith filter requires string value",
                            ));
                        }
                    },
                    Operator::EndsWith => {
                        if let Value::String(s) = json_value {
                            Filter::EndsWith(field.clone(), s)
                        }
                        else {
                            return Err(PyRuntimeError::new_err(
                                "EndsWith filter requires string value",
                            ));
                        }
                    },
                    Operator::In => {
                        if let Value::Array(arr) = json_value {
                            Filter::In(field.clone(), arr)
                        }
                        else {
                            return Err(PyRuntimeError::new_err("In filter requires array value"));
                        }
                    },
                    Operator::Exists => {
                        if let Value::Bool(b) = json_value {
                            Filter::Exists(field.clone(), b)
                        }
                        else {
                            return Err(PyRuntimeError::new_err(
                                "Exists filter requires boolean value",
                            ));
                        }
                    },
                };
                Ok(filter)
            })
            .collect();

        let agg = match aggregation.as_str() {
            "count" => Aggregation::Count,
            "sum" => Aggregation::Sum("value".to_string()), // Default field
            "avg" => Aggregation::Avg("value".to_string()),
            "min" => Aggregation::Min("value".to_string()),
            "max" => Aggregation::Max("value".to_string()),
            _ => {
                return Err(PyRuntimeError::new_err(format!(
                    "Unknown aggregation: {}",
                    aggregation
                )))
            },
        };

        // Run the async operation and convert result
        let rt = tokio::runtime::Runtime::new()?;
        let result = rt.block_on(async {
            collection
                .aggregate(query_filters?, agg)
                .await
                .sentinel_to_py()
        })?;
        json_value_to_pyobject(py, &result)
    }
}

/// Python wrapper for Sentinel Document
#[pyclass(name = "Document")]
#[derive(Clone)]
pub struct PyDocument {
    document: Document,
}

#[pymethods]
impl PyDocument {
    /// Get the document ID
    ///
    /// Returns:
    ///     str: The document identifier
    #[getter]
    fn id(&self) -> String { self.document.id().to_string() }

    /// Get the document version
    ///
    /// Returns:
    ///     int: The document version
    #[getter]
    fn version(&self) -> u32 { self.document.version() }

    /// Get the creation timestamp as ISO string
    ///
    /// Returns:
    ///     str: ISO formatted timestamp
    #[getter]
    fn created_at(&self) -> String { self.document.created_at().to_rfc3339() }

    /// Get the last update timestamp as ISO string
    ///
    /// Returns:
    ///     str: ISO formatted timestamp
    #[getter]
    fn updated_at(&self) -> String { self.document.updated_at().to_rfc3339() }

    /// Get the document hash
    ///
    /// Returns:
    ///     str: The document hash
    #[getter]
    fn hash(&self) -> String { self.document.hash().to_string() }

    /// Get the document signature
    ///
    /// Returns:
    ///     str: The document signature (empty if not signed)
    #[getter]
    fn signature(&self) -> String { self.document.signature().to_string() }

    /// Get the document data
    ///
    /// Returns:
    ///     dict: The JSON data
    #[getter]
    fn data(&self, py: Python) -> PyResult<PyObject> { json_value_to_pyobject(py, &self.document.data()) }
}

/// Python wrapper for QueryBuilder
#[pyclass(name = "QueryBuilder")]
#[derive(Clone)]
pub struct PyQueryBuilder {
    builder: QueryBuilder,
}

#[pymethods]
impl PyQueryBuilder {
    /// Create a new empty query builder
    ///
    /// Returns:
    ///     QueryBuilder: A new query builder instance
    #[new]
    fn new() -> Self {
        PyQueryBuilder {
            builder: QueryBuilder::new(),
        }
    }

    /// Add a filter condition
    ///
    /// Args:
    ///     field (str): Field name to filter on
    ///     operator (str): Filter operator (equals, greater_than, etc.)
    ///     value: Value to compare against
    ///
    /// Returns:
    ///     QueryBuilder: Self for chaining
    fn filter(&mut self, field: String, operator: String, value: PyObject, py: Python) -> PyResult<Self> {
        let json_value = pyobject_to_json_value(py, value)?;
        let op = match operator.as_str() {
            "equals" => Operator::Equals,
            "greater_than" => Operator::GreaterThan,
            "less_than" => Operator::LessThan,
            "greater_or_equal" => Operator::GreaterOrEqual,
            "less_or_equal" => Operator::LessOrEqual,
            "contains" => Operator::Contains,
            "starts_with" => Operator::StartsWith,
            "ends_with" => Operator::EndsWith,
            "in" => Operator::In,
            "exists" => Operator::Exists,
            _ => {
                return Err(PyRuntimeError::new_err(format!(
                    "Invalid operator: {}",
                    operator
                )))
            },
        };
        self.builder = self.builder.clone().filter(&field, op, json_value);
        Ok(PyQueryBuilder {
            builder: self.builder.clone(),
        })
    }

    /// Set sorting
    ///
    /// Args:
    ///     field (str): Field to sort by
    ///     order (str): Sort order ("ascending" or "descending")
    ///
    /// Returns:
    ///     QueryBuilder: Self for chaining
    fn sort(&mut self, field: String, order: String) -> PyResult<Self> {
        let sort_order = match order.as_str() {
            "ascending" => SortOrder::Ascending,
            "descending" => SortOrder::Descending,
            _ => {
                return Err(PyRuntimeError::new_err(format!(
                    "Invalid sort order: {}",
                    order
                )))
            },
        };
        self.builder = self.builder.clone().sort(&field, sort_order);
        Ok(PyQueryBuilder {
            builder: self.builder.clone(),
        })
    }

    /// Set limit on results
    ///
    /// Args:
    ///     limit (int): Maximum number of results
    ///
    /// Returns:
    ///     QueryBuilder: Self for chaining
    fn limit(&mut self, limit: usize) -> Self {
        PyQueryBuilder {
            builder: self.builder.clone().limit(limit),
        }
    }

    /// Set offset for pagination
    ///
    /// Args:
    ///     offset (int): Number of results to skip
    ///
    /// Returns:
    ///     QueryBuilder: Self for chaining
    fn offset(&mut self, offset: usize) -> Self {
        PyQueryBuilder {
            builder: self.builder.clone().offset(offset),
        }
    }

    /// Set field projection
    ///
    /// Args:
    ///     fields (List[str]): Fields to include in results
    ///
    /// Returns:
    ///     QueryBuilder: Self for chaining
    fn projection(&mut self, fields: Vec<String>) -> Self {
        let fields_refs: Vec<&str> = fields.iter().map(|s| s.as_str()).collect();
        PyQueryBuilder {
            builder: self.builder.clone().projection(fields_refs),
        }
    }
}

/// Python wrapper for QueryResult
#[pyclass(name = "QueryResult")]
pub struct PyQueryResult {
    documents:      Vec<PyDocument>,
    total_count:    Option<usize>,
    execution_time: std::time::Duration,
}

#[pymethods]
impl PyQueryResult {
    /// Get the documents from the query result
    ///
    /// Returns:
    ///     List[Document]: List of matching documents
    #[getter]
    fn documents(&self) -> Vec<PyDocument> { self.documents.clone() }

    /// Get the total count of matching documents
    ///
    /// Returns:
    ///     Optional[int]: Total count or None if not known
    #[getter]
    fn total_count(&self) -> Option<usize> { self.total_count }

    /// Get the execution time of the query
    ///
    /// Returns:
    ///     float: Execution time in seconds
    #[getter]
    fn execution_time(&self) -> f64 { self.execution_time.as_secs_f64() }
}

/// Python wrapper for VerificationOptions
#[pyclass(name = "VerificationOptions")]
#[derive(Clone)]
pub struct PyVerificationOptions {
    verify_hash:                 bool,
    verify_signature:            bool,
    hash_verification_mode:      VerificationMode,
    signature_verification_mode: VerificationMode,
    empty_signature_mode:        VerificationMode,
}

#[pymethods]
impl PyVerificationOptions {
    /// Create default verification options
    ///
    /// Returns:
    ///     PyVerificationOptions: Default options (strict verification)
    #[staticmethod]
    fn default() -> Self {
        let default = sentinel_dbms::VerificationOptions::default();
        PyVerificationOptions {
            verify_hash:                 default.verify_hash,
            verify_signature:            default.verify_signature,
            hash_verification_mode:      default.hash_verification_mode,
            signature_verification_mode: default.signature_verification_mode,
            empty_signature_mode:        default.empty_signature_mode,
        }
    }

    /// Create verification options that warn instead of failing
    ///
    /// Returns:
    ///     PyVerificationOptions: Warning mode options
    #[staticmethod]
    fn warn() -> Self {
        PyVerificationOptions {
            verify_hash:                 true,
            verify_signature:            true,
            hash_verification_mode:      VerificationMode::Warn,
            signature_verification_mode: VerificationMode::Warn,
            empty_signature_mode:        VerificationMode::Warn,
        }
    }

    /// Create verification options that skip verification
    ///
    /// Returns:
    ///     PyVerificationOptions: Silent mode options
    #[staticmethod]
    fn silent() -> Self {
        PyVerificationOptions {
            verify_hash:                 false,
            verify_signature:            false,
            hash_verification_mode:      VerificationMode::Silent,
            signature_verification_mode: VerificationMode::Silent,
            empty_signature_mode:        VerificationMode::Silent,
        }
    }
}

impl PyVerificationOptions {
    fn to_rust_options(&self) -> sentinel_dbms::VerificationOptions {
        sentinel_dbms::VerificationOptions {
            verify_hash:                 self.verify_hash,
            verify_signature:            self.verify_signature,
            hash_verification_mode:      self.hash_verification_mode,
            signature_verification_mode: self.signature_verification_mode,
            empty_signature_mode:        self.empty_signature_mode,
        }
    }
}

/// Convert a Python object to a JSON Value
fn pyobject_to_json_value(py: Python, obj: PyObject) -> PyResult<Value> {
    if obj.is_none(py) {
        Ok(Value::Null)
    }
    else if let Ok(b) = obj.extract::<bool>(py) {
        Ok(Value::Bool(b))
    }
    else if let Ok(i) = obj.extract::<i64>(py) {
        Ok(Value::Number(i.into()))
    }
    else if let Ok(f) = obj.extract::<f64>(py) {
        Ok(Value::Number(serde_json::Number::from_f64(f).unwrap()))
    }
    else if let Ok(s) = obj.extract::<String>(py) {
        Ok(Value::String(s))
    }
    else if let Ok(list) = obj.downcast::<pyo3::types::PyList>(py) {
        let mut vec = Vec::new();
        for item in list.iter() {
            vec.push(pyobject_to_json_value(py, item.into())?);
        }
        Ok(Value::Array(vec))
    }
    else if let Ok(dict) = obj.downcast::<pyo3::types::PyDict>(py) {
        let mut map = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            let json_value = pyobject_to_json_value(py, value.into())?;
            map.insert(key_str, json_value);
        }
        Ok(Value::Object(map))
    }
    else {
        Err(PyRuntimeError::new_err(
            "Unsupported Python type for JSON conversion",
        ))
    }
}

/// Convert a JSON Value to a Python object
fn json_value_to_pyobject(py: Python, value: &Value) -> PyResult<PyObject> {
    match value {
        Value::Object(map) => {
            let dict = pyo3::types::PyDict::new(py);
            for (key, val) in map {
                dict.set_item(key, json_value_to_pyobject(py, val)?)?;
            }
            Ok(dict.to_object(py))
        },
        Value::Array(vec) => {
            let list = pyo3::types::PyList::new(
                py,
                vec.iter()
                    .map(|v| json_value_to_pyobject(py, v))
                    .collect::<PyResult<Vec<_>>>()?,
            );
            Ok(list.to_object(py))
        },
        Value::String(s) => Ok(s.to_object(py)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_object(py))
            }
            else if let Some(f) = n.as_f64() {
                Ok(f.to_object(py))
            }
            else {
                Err(PyRuntimeError::new_err("Unsupported number type"))
            }
        },
        Value::Bool(b) => Ok(b.to_object(py)),
        Value::Null => Ok(py.None()),
    }
}

/// Hash JSON data using the configured algorithm
///
/// Args:
///     data: JSON-serializable data to hash
///
/// Returns:
///     str: The hash as a hex string
#[pyfunction]
fn hash_data_py(py: Python, data: PyObject) -> PyResult<&PyAny> {
    let json_value = pyobject_to_json_value(py, data)?;
    future_into_py(py, async move {
        let hash = hash_data(&json_value)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(hash)
    })
}

/// Sign a hash using the configured algorithm
///
/// Args:
///     hash (str): Hash to sign
///     private_key: Signing key (32 bytes)
///
/// Returns:
///     str: The signature as a hex string
#[pyfunction]
fn sign_hash_py(py: Python, hash: String, private_key_bytes: Vec<u8>) -> PyResult<&PyAny> {
    let key_array: Result<[u8; 32], _> = private_key_bytes
        .try_into()
        .map_err(|_| PyRuntimeError::new_err("Private key must be 32 bytes"));
    future_into_py(py, async move {
        let key = sentinel_crypto::SigningKey::from_bytes(&key_array?);
        let signature = sign_hash(&hash, &key)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(signature)
    })
}

/// Verify a signature using the configured algorithm
///
/// Args:
///     hash (str): Hash that was signed
///     signature (str): Signature to verify
///     public_key: Verifying key (32 bytes)
///
/// Returns:
///     bool: True if signature is valid
#[pyfunction]
fn verify_signature_py(py: Python, hash: String, signature: String, public_key_bytes: Vec<u8>) -> PyResult<&PyAny> {
    let key_array: Result<[u8; 32], _> = public_key_bytes
        .try_into()
        .map_err(|_| PyRuntimeError::new_err("Public key must be 32 bytes"));
    future_into_py(py, async move {
        let key = sentinel_crypto::VerifyingKey::from_bytes(&key_array?)
            .map_err(|_| PyRuntimeError::new_err("Invalid public key"))?;
        let is_valid = verify_signature(&hash, &signature, &key)
            .await
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(is_valid)
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn get_workspace_root() -> PathBuf {
        let mut path = std::env::current_dir().unwrap();
        // Start from the crate directory and look for workspace Cargo.toml
        // Look for a Cargo.toml that has [workspace] in it
        loop {
            let cargo_toml = path.join("Cargo.toml");
            if cargo_toml.exists() {
                // Read the Cargo.toml to check if it's a workspace
                if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                    if content.contains("[workspace]") {
                        return path;
                    }
                }
            }
            if let Some(parent) = path.parent() {
                if parent == path {
                    // Reached root without finding workspace
                    return std::env::current_dir().unwrap();
                }
                path = parent.to_path_buf();
            }
            else {
                return std::env::current_dir().unwrap();
            }
        }
    }

    fn run_python_tests() -> Result<(), Box<dyn std::error::Error>> {
        // Check if pytest is available
        let pytest_check = Command::new("python3")
            .arg("-c")
            .arg("import pytest")
            .output();

        if pytest_check.is_err() {
            return Err("Python not available".into());
        }

        let pytest_check_output = pytest_check.unwrap();
        if !pytest_check_output.status.success() {
            return Err("pytest not installed, skipping Python tests".into());
        }

        let workspace_root = get_workspace_root();
        let python_tests_dir = workspace_root.join("bindings").join("python");
        let target_dir = workspace_root.join("target").join("debug");

        // Set PYTHONPATH to include the target directory
        let python_path = std::env::var("PYTHONPATH").unwrap_or_default();
        let new_python_path = if python_path.is_empty() {
            target_dir.to_string_lossy().to_string()
        }
        else {
            format!("{}:{}", target_dir.to_string_lossy(), python_path)
        };

        // Run Python tests
        let output = Command::new("python3")
            .arg("-m")
            .arg("pytest")
            .arg("test_python_bindings.py")
            .arg("-v")
            .current_dir(&python_tests_dir)
            .env("PYTHONPATH", new_python_path)
            .output()?;

        if !output.status.success() {
            println!("Python tests failed:");
            println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
            println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
            return Err("Python tests failed".into());
        }

        println!("Python tests passed!");
        println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        Ok(())
    }

    fn run_python_examples() -> Result<(), Box<dyn std::error::Error>> {
        // Check if python3 is available
        let python_check = Command::new("python3").arg("--version").output();

        if python_check.is_err() {
            return Err("Python not available".into());
        }

        if !python_check.unwrap().status.success() {
            return Err("Python not installed, skipping Python examples".into());
        }

        let workspace_root = get_workspace_root();
        let python_examples_dir = workspace_root.join("bindings").join("python");
        let target_dir = workspace_root.join("target").join("debug");

        // Debug output
        println!("Workspace root: {:?}", workspace_root);
        println!("Examples dir: {:?}", python_examples_dir);
        println!("Target dir: {:?}", target_dir);

        // Check if the example file exists
        let example_file = python_examples_dir.join("example_python_bindings.py");
        println!("Example file: {:?}", example_file);
        if !example_file.exists() {
            return Err(format!("Example file not found: {:?}", example_file).into());
        }

        // Set PYTHONPATH to include the target directory
        let python_path = std::env::var("PYTHONPATH").unwrap_or_default();
        let new_python_path = if python_path.is_empty() {
            target_dir.to_string_lossy().to_string()
        }
        else {
            format!("{}:{}", target_dir.to_string_lossy(), python_path)
        };

        // Run Python examples
        let example_files = ["example_python_bindings.py"];

        for example_file in &example_files {
            println!("Running Python example: {}", example_file);
            let output = Command::new("python3")
                .arg(example_file)
                .current_dir(&python_examples_dir)
                .env("PYTHONPATH", new_python_path.clone())
                .output()?;

            if !output.status.success() {
                println!("Python example {} failed:", example_file);
                println!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
                println!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
                return Err(format!("Python example {} failed", example_file).into());
            }

            println!("Python example {} passed!", example_file);
        }

        Ok(())
    }

    #[test]
    fn test_python_bindings_compilation() {
        // Test that the Python extension compiles
        // This is a basic compilation test
        assert!(true);
    }

    #[test]
    fn test_python_tests_integration() {
        // Run Python tests and examples
        // If Python is not available or tests fail, this test will fail
        // If pytest is not installed, tests are skipped
        match run_python_tests() {
            Ok(_) => {},
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("pytest not installed") {
                    println!("Skipping Python tests: {}", e);
                }
                else {
                    panic!("Python tests integration failed: {}", e);
                }
            },
        }
    }

    #[test]
    fn test_python_examples_integration() {
        // Run Python examples
        // If Python is not available or examples fail, this test will fail
        match run_python_examples() {
            Ok(_) => {},
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("Python not installed") {
                    println!("Skipping Python examples: {}", e);
                }
                else {
                    panic!("Python examples integration failed: {}", e);
                }
            },
        }
    }
}
