use std::{fs, io, path::PathBuf};

use wasm_bindgen::prelude::*;
use js_sys::Array;
use serde_json::Value;
use chrono::Utc;
use sha2::{Digest, Sha256};
use hex;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

fn compute_hash(data: &Value) -> String {
    let json = serde_json::to_string(data).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(json);
    hex::encode(hasher.finalize())
}

struct Collection {
    name: String,
    path: PathBuf,
}

impl Collection {
    fn new(name: &str, base_path: &PathBuf) -> Self {
        let path = base_path.join(name);
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        Self {
            name: name.to_string(),
            path,
        }
    }

    fn insert(&self, id: &str, data: Value) -> io::Result<()> {
        let hash = compute_hash(&data);
        let version = 1;
        let created_at = Utc::now();
        let updated_at = Utc::now();

        let doc = serde_json::json!({
            "id": id,
            "version": version,
            "created_at": created_at.to_rfc3339(),
            "updated_at": updated_at.to_rfc3339(),
            "hash": hash,
            "signature": "",
            "data": data
        });

        let file_path = self.path.join(format!("{}.json", sanitize_id(id)));
        fs::write(&file_path, serde_json::to_string_pretty(&doc)?)?;
        Ok(())
    }

    fn get(&self, id: &str) -> io::Result<Option<Value>> {
        let file_path = self.path.join(format!("{}.json", sanitize_id(id)));
        if !file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&file_path)?;
        let doc: Value = serde_json::from_str(&content)?;
        Ok(Some(doc["data"].clone()))
    }

    fn delete(&self, id: &str) -> io::Result<()> {
        let file_path = self.path.join(format!("{}.json", sanitize_id(id)));
        if file_path.exists() {
            fs::remove_file(&file_path)?;
        }
        Ok(())
    }

    fn count(&self) -> io::Result<u32> {
        let entries = fs::read_dir(&self.path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "json")
                    .unwrap_or(false)
            })
            .count();
        Ok(entries as u32)
    }

    fn list(&self) -> io::Result<Vec<String>> {
        let mut ids = Vec::new();
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;
            if entry
                .path()
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false)
            {
                if let Some(name) = entry.file_name().to_str() {
                    if let Some(id) = name.strip_suffix(".json") {
                        ids.push(id.to_string());
                    }
                }
            }
        }
        Ok(ids)
    }

    fn all(&self) -> io::Result<Vec<Value>> {
        let mut docs = Vec::new();
        for id in self.list()? {
            if let Some(doc) = self.get(&id)? {
                docs.push(doc);
            }
        }
        Ok(docs)
    }
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            }
            else {
                '_'
            }
        })
        .collect()
}

#[wasm_bindgen]
pub struct WasmStore {
    base_path: PathBuf,
}

#[wasm_bindgen]
impl WasmStore {
    #[wasm_bindgen(constructor)]
    pub fn new(path: &str) -> Result<WasmStore, JsValue> {
        Ok(WasmStore {
            base_path: PathBuf::from(path),
        })
    }

    pub fn create(path: &str) -> Result<WasmStore, JsValue> {
        let store = WasmStore {
            base_path: PathBuf::from(path),
        };

        if !store.base_path.exists() {
            let _ = fs::create_dir_all(&store.base_path).map_err(|e| e.to_string())?;
        }

        Ok(store)
    }

    pub fn collection(&self, name: &str) -> Result<WasmCollection, JsValue> {
        Ok(WasmCollection {
            collection: Arc::new(Collection::new(name, &self.base_path)),
        })
    }

    pub fn delete_collection(&self, name: &str) -> Result<(), JsValue> {
        let path = self.base_path.join(name);
        if path.exists() {
            fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn list_collections(&self) -> Result<JsValue, JsValue> {
        let array = Array::new();
        if self.base_path.exists() {
            for entry in fs::read_dir(&self.base_path).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        array.push(&JsValue::from_str(name));
                    }
                }
            }
        }
        Ok(array.into())
    }
}

use std::sync::Arc;

#[wasm_bindgen]
pub struct WasmCollection {
    collection: Arc<Collection>,
}

#[wasm_bindgen]
impl WasmCollection {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String { self.collection.name.clone() }

    pub fn insert(&self, id: &str, data: JsValue) -> Result<(), JsValue> {
        let json_str = js_sys::JSON::stringify(&data)
            .map_err(|e| e)?
            .as_string()
            .ok_or_else(|| JsValue::from_str("Failed to stringify JSON"))?;
        let json_value: Value =
            serde_json::from_str(&json_str).map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))?;

        self.collection
            .insert(id, json_value)
            .map_err(|e| JsValue::from_str(&format!("Failed to insert: {}", e)))
    }

    pub fn get(&self, id: &str) -> Result<JsValue, JsValue> {
        match self.collection.get(id).map_err(|e| e.to_string())? {
            Some(data) => {
                let json_str =
                    serde_json::to_string(&data).map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))?;
                js_sys::JSON::parse(&json_str).map_err(|e| e)
            },
            None => Ok(JsValue::NULL),
        }
    }

    pub fn delete(&self, id: &str) -> Result<(), JsValue> {
        self.collection
            .delete(id)
            .map_err(|e| JsValue::from_str(&format!("Failed to delete: {}", e)))
    }

    pub fn count(&self) -> Result<u32, JsValue> {
        self.collection
            .count()
            .map_err(|e| JsValue::from_str(&format!("Failed to count: {}", e)))
    }

    pub fn update(&self, id: &str, data: JsValue) -> Result<(), JsValue> { self.insert(id, data) }

    pub fn upsert(&self, id: &str, data: JsValue) -> Result<bool, JsValue> {
        let exists = self
            .collection
            .get(id)
            .map_err(|e| e.to_string())?
            .is_some();
        self.insert(id, data)?;
        Ok(!exists)
    }

    pub fn bulk_insert(&self, documents: JsValue) -> Result<(), JsValue> {
        let array = documents
            .dyn_ref::<Array>()
            .ok_or_else(|| JsValue::from_str("Expected array"))?;

        for i in 0 .. array.length() {
            let item = array.get(i);
            let id = js_sys::Reflect::get(&item, &"id".into())
                .map_err(|e| e)?
                .as_string()
                .ok_or_else(|| JsValue::from_str("Missing id field"))?;
            self.insert(&id, item)?;
        }

        Ok(())
    }

    pub fn list(&self) -> Result<JsValue, JsValue> {
        let array = Array::new();
        for id in self
            .collection
            .list()
            .map_err(|e| JsValue::from_str(&format!("Failed to list: {}", e)))?
        {
            array.push(&JsValue::from_str(&id));
        }
        Ok(array.into())
    }

    pub fn all(&self) -> Result<JsValue, JsValue> {
        let array = Array::new();
        for doc in self
            .collection
            .all()
            .map_err(|e| JsValue::from_str(&format!("Failed to get all: {}", e)))?
        {
            let json_str = serde_json::to_string(&doc).map_err(|e| JsValue::from_str(&format!("JSON error: {}", e)))?;
            let js_doc = js_sys::JSON::parse(&json_str).map_err(|e| e)?;
            array.push(&js_doc);
        }
        Ok(array.into())
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    console_log!("Cyberpath Sentinel WebAssembly module initialized");
}
