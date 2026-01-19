use std::collections::HashMap;

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
            json_docs.push((id, json_value));
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
    fn id(&self) -> String { self.document.id.clone() }

    /// Get the document data as a Python dict
    #[getter]
    fn data(&self, py: Python) -> PyResult<PyObject> { json_value_to_pyobject(py, &self.document.data) }

    /// Get the document version
    #[getter]
    fn version(&self) -> u32 { self.document.version }

    /// Get the document creation timestamp
    #[getter]
    fn created_at(&self) -> String { self.document.created_at.to_rfc3339() }

    /// Get the document last update timestamp
    #[getter]
    fn updated_at(&self) -> String { self.document.updated_at.to_rfc3339() }
}

/// Convert a Python object to a JSON Value
fn pyobject_to_json_value(py: Python, obj: PyObject) -> PyResult<Value> {
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
    else if let Ok(string) = obj.extract::<String>(py) {
        Ok(Value::String(string))
    }
    else if let Ok(int) = obj.extract::<i64>(py) {
        Ok(Value::Number(int.into()))
    }
    else if let Ok(float) = obj.extract::<f64>(py) {
        Ok(Value::Number(serde_json::Number::from_f64(float).unwrap()))
    }
    else if let Ok(bool) = obj.extract::<bool>(py) {
        Ok(Value::Bool(bool))
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
