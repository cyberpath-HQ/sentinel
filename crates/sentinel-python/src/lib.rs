use pyo3::{exceptions::PyRuntimeError, prelude::*};
use pyo3_asyncio::tokio::future_into_py;
use sentinel_dbms::{Collection, Document, Store};
use serde_json::Value;

/// Python bindings for Cyberpath Sentinel DBMS
#[pymodule]
fn sentinel(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyStore>()?;
    m.add_class::<PyCollection>()?;
    m.add_class::<PyDocument>()?;
    Ok(())
}

/// Python wrapper for Sentinel Store
#[pyclass(name = "Store")]
pub struct PyStore {
    store: Store,
}

#[pymethods]
impl PyStore {
    /// Create a new Sentinel store
    #[staticmethod]
    #[pyo3(signature = (path, passphrase=None))]
    fn new(py: Python, path: String, passphrase: Option<String>) -> PyResult<PyObject> {
        let passphrase = passphrase.as_deref();

        future_into_py(py, async move {
            match Store::new(&path, passphrase).await {
                Ok(store) => {
                    Ok(PyStore {
                        store,
                    })
                },
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to create store: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Get a collection from the store
    fn collection(&self, py: Python, name: String) -> PyResult<PyObject> {
        let store = &self.store;

        future_into_py(py, async move {
            match store.collection(&name).await {
                Ok(collection) => {
                    Ok(PyCollection {
                        collection,
                    })
                },
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to get collection: {}",
                        e
                    )))
                },
            }
        })
    }

    /// List all collections in the store
    fn list_collections(&self, py: Python) -> PyResult<PyObject> {
        let store = &self.store;

        future_into_py(py, async move {
            match store.list_collections().await {
                Ok(collections) => Ok(collections),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to list collections: {}",
                        e
                    )))
                },
            }
        })
    }
}

/// Python wrapper for Sentinel Collection
#[pyclass(name = "Collection")]
pub struct PyCollection {
    collection: Collection,
}

#[pymethods]
impl PyCollection {
    /// Insert a document into the collection
    fn insert(&self, py: Python, id: String, data: PyObject) -> PyResult<PyObject> {
        let collection = &self.collection;
        let json_value = pyobject_to_json_value(py, data)?;

        future_into_py(py, async move {
            match collection.insert(&id, json_value).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to insert document: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Get a document by ID
    fn get(&self, py: Python, id: String) -> PyResult<PyObject> {
        let collection = &self.collection;

        future_into_py(py, async move {
            match collection.get(&id).await {
                Ok(Some(doc)) => {
                    Ok(Some(PyDocument {
                        document: doc,
                    }))
                },
                Ok(None) => Ok(None),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to get document: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Delete a document by ID
    fn delete(&self, py: Python, id: String) -> PyResult<PyObject> {
        let collection = &self.collection;

        future_into_py(py, async move {
            match collection.delete(&id).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to delete document: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Get the count of documents in the collection
    fn count(&self, py: Python) -> PyResult<PyObject> {
        let collection = &self.collection;

        future_into_py(py, async move {
            match collection.count().await {
                Ok(count) => Ok(count),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to count documents: {}",
                        e
                    )))
                },
            }
        })
    }
}

/// Python wrapper for Sentinel Document
#[pyclass(name = "Document")]
pub struct PyDocument {
    document: Document,
}

#[pymethods]
impl PyDocument {
    /// Get the document ID
    #[getter]
    fn id(&self) -> String { self.document.id().to_string() }

    /// Get the document data as a Python dict
    #[getter]
    fn data(&self, py: Python) -> PyResult<PyObject> { json_value_to_pyobject(py, &self.document.data()) }
}

/// Convert a Python object to a JSON Value
fn pyobject_to_json_value(py: Python, obj: PyObject) -> PyResult<Value> {
    // Simple conversion - try to extract as JSON string first
    if let Ok(json_str) = obj.extract::<String>(py) {
        if let Ok(value) = serde_json::from_str(&json_str) {
            return Ok(value);
        }
    }

    // Fallback: convert Python objects to JSON
    if let Ok(dict) = obj.downcast::<pyo3::types::PyDict>(py) {
        let mut map = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            let json_value = pyobject_to_json_value(py, value)?;
            map.insert(key_str, json_value);
        }
        Ok(Value::Object(map))
    }
    else if let Ok(list) = obj.downcast::<pyo3::types::PyList>(py) {
        let mut vec = Vec::new();
        for item in list.iter() {
            vec.push(pyobject_to_json_value(py, item)?);
        }
        Ok(Value::Array(vec))
    }
    else if let Ok(s) = obj.extract::<String>(py) {
        Ok(Value::String(s))
    }
    else if let Ok(i) = obj.extract::<i64>(py) {
        Ok(Value::Number(i.into()))
    }
    else if let Ok(f) = obj.extract::<f64>(py) {
        Ok(Value::Number(serde_json::Number::from_f64(f).unwrap()))
    }
    else if let Ok(b) = obj.extract::<bool>(py) {
        Ok(Value::Bool(b))
    }
    else if obj.is_none(py) {
        Ok(Value::Null)
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

/// Python wrapper for Sentinel Store
#[pyclass(name = "Store")]
pub struct PyStore {
    store: Store,
}

#[pymethods]
impl PyStore {
    /// Create a new Sentinel store
    #[staticmethod]
    #[pyo3(signature = (path, passphrase=None))]
    fn new(py: Python, path: String, passphrase: Option<String>) -> PyResult<PyObject> {
        let passphrase = passphrase.as_deref();

        future_into_py(py, async move {
            match Store::new(&path, passphrase).await {
                Ok(store) => {
                    Ok(PyStore {
                        store,
                    })
                },
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to create store: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Get a collection from the store
    fn collection(&self, py: Python, name: String) -> PyResult<PyObject> {
        let store = &self.store;

        future_into_py(py, async move {
            match store.collection(&name).await {
                Ok(collection) => {
                    Ok(PyCollection {
                        collection,
                    })
                },
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to get collection: {}",
                        e
                    )))
                },
            }
        })
    }

    /// List all collections in the store
    fn list_collections(&self, py: Python) -> PyResult<PyObject> {
        let store = &self.store;

        future_into_py(py, async move {
            match store.list_collections().await {
                Ok(collections) => Ok(collections),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to list collections: {}",
                        e
                    )))
                },
            }
        })
    }
}

/// Python wrapper for Sentinel Collection
#[pyclass(name = "Collection")]
pub struct PyCollection {
    collection: Collection,
}

#[pymethods]
impl PyCollection {
    /// Insert a document into the collection
    fn insert(&self, py: Python, id: String, data: PyObject) -> PyResult<PyObject> {
        let collection = &self.collection;
        let json_value = pyobject_to_json_value(py, data)?;

        future_into_py(py, async move {
            match collection.insert(&id, json_value).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to insert document: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Get a document by ID
    fn get(&self, py: Python, id: String) -> PyResult<PyObject> {
        let collection = &self.collection;

        future_into_py(py, async move {
            match collection.get(&id).await {
                Ok(Some(doc)) => {
                    Ok(Some(PyDocument {
                        document: doc,
                    }))
                },
                Ok(None) => Ok(None),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to get document: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Delete a document by ID
    fn delete(&self, py: Python, id: String) -> PyResult<PyObject> {
        let collection = &self.collection;

        future_into_py(py, async move {
            match collection.delete(&id).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to delete document: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Get the count of documents in the collection
    fn count(&self, py: Python) -> PyResult<PyObject> {
        let collection = &self.collection;

        future_into_py(py, async move {
            match collection.count().await {
                Ok(count) => Ok(count),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to count documents: {}",
                        e
                    )))
                },
            }
        })
    }
}

/// Python wrapper for Sentinel Document
#[pyclass(name = "Document")]
pub struct PyDocument {
    document: Document,
}

#[pymethods]
impl PyDocument {
    /// Get the document ID
    #[getter]
    fn id(&self) -> String { self.document.id().to_string() }

    /// Get the document data as a Python dict
    #[getter]
    fn data(&self, py: Python) -> PyResult<PyObject> { json_value_to_pyobject(py, &self.document.data()) }
}

/// Python wrapper for Sentinel Store
#[pyclass(name = "Store")]
pub struct PyStore {
    store: Store,
}

#[pymethods]
impl PyStore {
    /// Create a new Sentinel store
    #[staticmethod]
    #[pyo3(signature = (path, passphrase=None))]
    fn new(py: Python, path: String, passphrase: Option<String>) -> PyResult<PyObject> {
        let passphrase = passphrase.as_deref();

        future_into_py(py, async move {
            match Store::new(&path, passphrase).await {
                Ok(store) => {
                    Ok(PyStore {
                        store,
                    })
                },
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to create store: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Get a collection from the store
    #[pyo3(signature = (name))]
    fn collection(&self, py: Python, name: String) -> PyResult<PyObject> {
        let store = &self.store;

        future_into_py(py, async move {
            match store.collection(&name).await {
                Ok(collection) => {
                    Ok(PyCollection {
                        collection,
                    })
                },
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to get collection: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Delete a collection from the store
    #[pyo3(signature = (name))]
    fn delete_collection(&self, py: Python, name: String) -> PyResult<PyObject> {
        let store = &self.store;

        future_into_py(py, async move {
            match store.delete_collection(&name).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to delete collection: {}",
                        e
                    )))
                },
            }
        })
    }

    /// List all collections in the store
    fn list_collections(&self, py: Python) -> PyResult<PyObject> {
        let store = &self.store;

        future_into_py(py, async move {
            match store.list_collections().await {
                Ok(collections) => Ok(collections),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to list collections: {}",
                        e
                    )))
                },
            }
        })
    }
}

/// Python wrapper for Sentinel Collection
#[pyclass(name = "Collection")]
pub struct PyCollection {
    collection: Collection,
}

#[pymethods]
impl PyCollection {
    /// Insert a document into the collection
    #[pyo3(signature = (id, data))]
    fn insert(&self, py: Python, id: String, data: PyObject) -> PyResult<PyObject> {
        let collection = &self.collection;
        let json_value = pyobject_to_json_value(py, data)?;

        future_into_py(py, async move {
            match collection.insert(&id, json_value).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to insert document: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Get a document by ID
    #[pyo3(signature = (id))]
    fn get(&self, py: Python, id: String) -> PyResult<PyObject> {
        let collection = &self.collection;

        future_into_py(py, async move {
            match collection.get(&id).await {
                Ok(Some(doc)) => {
                    Ok(Some(PyDocument {
                        document: doc,
                    }))
                },
                Ok(None) => Ok(None),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to get document: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Delete a document by ID
    #[pyo3(signature = (id))]
    fn delete(&self, py: Python, id: String) -> PyResult<PyObject> {
        let collection = &self.collection;

        future_into_py(py, async move {
            match collection.delete(&id).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to delete document: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Get the count of documents in the collection
    fn count(&self, py: Python) -> PyResult<PyObject> {
        let collection = &self.collection;

        future_into_py(py, async move {
            match collection.count().await {
                Ok(count) => Ok(count),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to count documents: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Bulk insert multiple documents
    #[pyo3(signature = (documents))]
    fn bulk_insert(&self, py: Python, documents: Vec<(String, PyObject)>) -> PyResult<PyObject> {
        let collection = &self.collection;

        let mut json_docs = Vec::new();
        for (id, data) in documents {
            let json_value = match pyobject_to_json_value(py, data) {
                Ok(v) => v,
                Err(e) => return Err(e),
            };
            json_docs.push((&id, json_value));
        }

        future_into_py(py, async move {
            match collection.bulk_insert(json_docs).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to bulk insert: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Update a document
    #[pyo3(signature = (id, data))]
    fn update(&self, py: Python, id: String, data: PyObject) -> PyResult<PyObject> {
        let collection = &self.collection;
        let json_value = pyobject_to_json_value(py, data)?;

        future_into_py(py, async move {
            match collection.update(&id, json_value).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to update document: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Upsert a document (insert or update)
    #[pyo3(signature = (id, data))]
    fn upsert(&self, py: Python, id: String, data: PyObject) -> PyResult<PyObject> {
        let collection = &self.collection;
        let json_value = pyobject_to_json_value(py, data)?;

        future_into_py(py, async move {
            match collection.upsert(&id, json_value).await {
                Ok(was_insert) => Ok(was_insert),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to upsert document: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Get multiple documents by IDs
    #[pyo3(signature = (ids))]
    fn get_many(&self, py: Python, ids: Vec<String>) -> PyResult<PyObject> {
        let collection = &self.collection;
        let ids_refs: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();

        future_into_py(py, async move {
            match collection.get_many(&ids_refs).await {
                Ok(documents) => {
                    let py_docs: Vec<Option<PyDocument>> = documents
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
                },
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to get documents: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Query documents with filters
    #[pyo3(signature = (filters=None, sort_by=None, sort_order="asc", limit=None, offset=None))]
    fn query(
        &self,
        py: Python,
        filters: Option<Vec<(String, String, PyObject)>>,
        sort_by: Option<String>,
        sort_order: Option<String>,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> PyResult<PyObject> {
        let collection = &self.collection;

        let query_filters = if let Some(filters) = filters {
            let mut q_filters = Vec::new();
            for (field, op, value) in filters {
                let operator = match op.as_str() {
                    "eq" => Operator::Equals,
                    "gt" => Operator::GreaterThan,
                    "gte" => Operator::GreaterOrEqual,
                    "lt" => Operator::LessThan,
                    "lte" => Operator::LessOrEqual,
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
                q_filters.push(filter);
            }
            q_filters
        }
        else {
            Vec::new()
        };

        let sort_order = match sort_order.as_deref() {
            Some("desc") => SortOrder::Descending,
            _ => SortOrder::Ascending,
        };

        let mut query_builder = QueryBuilder::new();
        for filter in filters {
            query_builder = query_builder.filter(filter.field.as_str(), filter.operator, filter.value);
        }

        if let Some(sort_field) = sort_by {
            query_builder = query_builder.sort_by(&sort_field, sort_order);
        }

        if let Some(limit) = limit {
            query_builder = query_builder.limit(limit);
        }

        if let Some(offset) = offset {
            query_builder = query_builder.offset(offset);
        }

        let query = query_builder.build();

        future_into_py(py, async move {
            match collection.query(query).await {
                Ok(result) => {
                    use futures::StreamExt;
                    let documents: Vec<PyDocument> = result
                        .documents
                        .collect::<Vec<_>>()
                        .await
                        .into_iter()
                        .filter_map(|res| res.ok())
                        .map(|doc| {
                            PyDocument {
                                document: doc,
                            }
                        })
                        .collect();

                    let py_result = pyo3::types::PyDict::new(py);
                    py_result.set_item("documents", documents)?;
                    py_result.set_item("total_count", result.total_count)?;
                    Ok(py_result.to_object(py))
                },
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to query documents: {}",
                        e
                    )))
                },
            }
        })
    }

    /// Aggregate documents
    #[pyo3(signature = (filters, aggregation))]
    fn aggregate(
        &self,
        py: Python,
        filters: Vec<(String, String, PyObject)>,
        aggregation: String,
    ) -> PyResult<PyObject> {
        let collection = &self.collection;

        let query_filters: Vec<Filter> = filters
            .into_iter()
            .map(|(field, op, value)| {
                let operator = match op.as_str() {
                    "eq" => Operator::Equals,
                    "gt" => Operator::GreaterThan,
                    "gte" => Operator::GreaterOrEqual,
                    "lt" => Operator::LessThan,
                    "lte" => Operator::LessOrEqual,
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
            .collect::<PyResult<Vec<_>>>()?;

        let aggregation = match aggregation.as_str() {
            "count" => sentinel_dbms::Aggregation::Count,
            "sum" => sentinel_dbms::Aggregation::Sum("".to_string()),
            "avg" => sentinel_dbms::Aggregation::Avg("".to_string()),
            "min" => sentinel_dbms::Aggregation::Min("".to_string()),
            "max" => sentinel_dbms::Aggregation::Max("".to_string()),
            _ => {
                return Err(PyRuntimeError::new_err(format!(
                    "Unknown aggregation: {}",
                    aggregation
                )))
            },
        };

        future_into_py(py, async move {
            match collection.aggregate(query_filters, aggregation).await {
                Ok(result) => json_value_to_pyobject(py, &result),
                Err(e) => {
                    Err(PyRuntimeError::new_err(format!(
                        "Failed to aggregate documents: {}",
                        e
                    )))
                },
            }
        })
    }
}

/// Python wrapper for Sentinel Document
#[pyclass(name = "Document")]
pub struct PyDocument {
    document: Document,
}

#[pymethods]
impl PyDocument {
    /// Get the document ID
    #[getter]
    fn id(&self) -> String { self.document.id().to_string() }

    /// Get the document data as a Python dict
    #[getter]
    fn data(&self, py: Python) -> PyResult<PyObject> { json_value_to_pyobject(py, &self.document.data()) }

    /// Get the document version
    #[getter]
    fn version(&self) -> u32 { self.document.version() }

    /// Get the document creation timestamp
    #[getter]
    fn created_at(&self) -> String { self.document.created_at().to_rfc3339() }

    /// Get the document last update timestamp
    #[getter]
    fn updated_at(&self) -> String { self.document.updated_at().to_rfc3339() }
}

/// Convert a Python object to a JSON Value
fn pyobject_to_json_value(_py: Python, obj: PyObject) -> PyResult<Value> {
    // For now, just extract as string and parse as JSON
    // This is a simplified implementation
    let json_str: String = obj.extract(_py)?;
    match serde_json::from_str(&json_str) {
        Ok(value) => Ok(value),
        Err(_) => Ok(Value::String(json_str)),
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
