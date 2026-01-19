use std::sync::Arc;

use pyo3::{exceptions::PyRuntimeError, prelude::*};
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
    store: Arc<Store>,
}

#[pymethods]
impl PyStore {
    /// Create a new Sentinel store
    #[staticmethod]
    #[pyo3(signature = (path, passphrase=None))]
    fn new(path: String, passphrase: Option<String>) -> PyResult<Self> {
        let rt = tokio::runtime::Runtime::new()?;
        let passphrase = passphrase.as_deref();

        let store = rt
            .block_on(async { Store::new(&path, passphrase).await })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to create store: {}", e)))?;

        Ok(PyStore {
            store: Arc::new(store),
        })
    }

    /// Get or create a collection with the specified name
    fn collection(&self, name: String) -> PyResult<PyCollection> {
        let rt = tokio::runtime::Runtime::new()?;
        let store = Arc::clone(&self.store);

        let collection = rt
            .block_on(async { store.collection(&name).await })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to get collection: {}", e)))?;

        Ok(PyCollection {
            collection: Arc::new(collection),
        })
    }

    /// List all collections in the store
    fn list_collections(&self) -> PyResult<Vec<String>> {
        let rt = tokio::runtime::Runtime::new()?;
        let store = Arc::clone(&self.store);

        let collections = rt
            .block_on(async { store.list_collections().await })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to list collections: {}", e)))?;

        Ok(collections)
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
    fn name(&self) -> String { self.collection.name().to_string() }

    /// Insert a new document or overwrite an existing one
    fn insert(&self, id: String, data: PyObject, py: Python) -> PyResult<()> {
        let rt = tokio::runtime::Runtime::new()?;
        let collection = Arc::clone(&self.collection);
        let json_value = pyobject_to_json_value(py, data)?;

        rt.block_on(async { collection.insert(&id, json_value).await })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to insert document: {}", e)))?;

        Ok(())
    }

    /// Retrieve a document by its ID
    fn get(&self, id: String) -> PyResult<Option<PyDocument>> {
        let rt = tokio::runtime::Runtime::new()?;
        let collection = Arc::clone(&self.collection);

        let doc = rt
            .block_on(async { collection.get(&id).await })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to get document: {}", e)))?;

        match doc {
            Some(document) => {
                Ok(Some(PyDocument {
                    document,
                }))
            },
            None => Ok(None),
        }
    }

    /// Count the total number of documents in the collection
    fn count(&self) -> PyResult<usize> {
        let rt = tokio::runtime::Runtime::new()?;
        let collection = Arc::clone(&self.collection);

        let count = rt
            .block_on(async { collection.count().await })
            .map_err(|e| PyRuntimeError::new_err(format!("Failed to count documents: {}", e)))?;

        Ok(count)
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

    /// Get the document version
    #[getter]
    fn version(&self) -> u32 { self.document.version() }

    /// Get the creation timestamp as ISO string
    #[getter]
    fn created_at(&self) -> String { self.document.created_at().to_rfc3339() }

    /// Get the last update timestamp as ISO string
    #[getter]
    fn updated_at(&self) -> String { self.document.updated_at().to_rfc3339() }

    /// Get the document hash
    #[getter]
    fn hash(&self) -> String { self.document.hash().to_string() }

    /// Get the document data
    #[getter]
    fn data(&self, py: Python) -> PyResult<PyObject> { json_value_to_pyobject(py, &self.document.data()) }
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
