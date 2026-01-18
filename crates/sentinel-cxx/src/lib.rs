use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    ptr,
    sync::Mutex,
};

use sentinel_dbms::{Collection, Filter, Query, Store};
use tokio::runtime::Runtime;
use serde_json::Value;
use once_cell::sync::Lazy;

// Global Tokio runtime for C API
static RUNTIME: Lazy<Mutex<Runtime>> =
    Lazy::new(|| Mutex::new(Runtime::new().expect("Failed to create Tokio runtime")));

// Error handling with thread-local storage
thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

fn set_error(err: impl std::fmt::Display) { LAST_ERROR.with(|e| *e.borrow_mut() = Some(err.to_string())); }

fn get_error() -> Option<String> { LAST_ERROR.with(|e| e.borrow().clone()) }

/// Opaque handle to a Sentinel Store
#[repr(C)]
pub struct sentinel_store_t {
    _private: [u8; 0],
}

/// Opaque handle to a Sentinel Collection
#[repr(C)]
pub struct sentinel_collection_t {
    _private: [u8; 0],
}

/// Opaque handle to a Sentinel Document
#[repr(C)]
pub struct sentinel_document_t {
    _private: [u8; 0],
}

/// Opaque handle to a Sentinel Query
#[repr(C)]
pub struct sentinel_query_t {
    _private: [u8; 0],
}

/// Error codes returned by Sentinel operations
#[repr(C)]
pub enum sentinel_error_t {
    SENTINEL_OK                     = 0,
    SENTINEL_ERROR_NULL_POINTER     = 1,
    SENTINEL_ERROR_INVALID_ARGUMENT = 2,
    SENTINEL_ERROR_IO_ERROR         = 3,
    SENTINEL_ERROR_RUNTIME_ERROR    = 4,
    SENTINEL_ERROR_JSON_PARSE_ERROR = 5,
    SENTINEL_ERROR_NOT_FOUND        = 6,
}

/// Create a new Sentinel store synchronously (blocking)
/// Returns NULL on error, check sentinel_get_last_error() for details
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_store_new(path: *const c_char, passphrase: *const c_char) -> *mut sentinel_store_t {
    if path.is_null() {
        set_error("Path cannot be null");
        return ptr::null_mut();
    }

    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in path: {}", e));
            return ptr::null_mut();
        },
    };

    let passphrase_str = if !passphrase.is_null() {
        match unsafe { CStr::from_ptr(passphrase) }.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => {
                set_error(format!("Invalid UTF-8 in passphrase: {}", e));
                return ptr::null_mut();
            },
        }
    }
    else {
        None
    };

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return ptr::null_mut();
        },
    };

    let store = match rt.block_on(Store::new(path_str, passphrase_str.as_deref())) {
        Ok(store) => store,
        Err(e) => {
            set_error(format!("Failed to create store: {}", e));
            return ptr::null_mut();
        },
    };

    Box::into_raw(Box::new(store)) as *mut sentinel_store_t
}

/// Free a Sentinel store
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_store_free(store: *mut sentinel_store_t) {
    if !store.is_null() {
        unsafe {
            let _ = Box::from_raw(store as *mut Store);
        }
    }
}

/// Get a collection from the store
/// Returns NULL on error, check sentinel_get_last_error() for details
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_store_collection(
    store: *mut sentinel_store_t,
    name: *const c_char,
) -> *mut sentinel_collection_t {
    if store.is_null() || name.is_null() {
        set_error("Store and name cannot be null");
        return ptr::null_mut();
    }

    let store_ref = unsafe { &*(store as *mut Store) };
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in collection name: {}", e));
            return ptr::null_mut();
        },
    };

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return ptr::null_mut();
        },
    };

    let collection = match rt.block_on(store_ref.collection(name_str)) {
        Ok(collection) => collection,
        Err(e) => {
            set_error(format!("Failed to get collection: {}", e));
            return ptr::null_mut();
        },
    };

    Box::into_raw(Box::new(collection)) as *mut sentinel_collection_t
}

/// Delete a collection from the store
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_store_delete_collection(
    store: *mut sentinel_store_t,
    name: *const c_char,
) -> sentinel_error_t {
    if store.is_null() || name.is_null() {
        set_error("Store and name cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let store_ref = unsafe { &*(store as *mut Store) };
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in collection name: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_RUNTIME_ERROR;
        },
    };

    match rt.block_on(store_ref.delete_collection(name_str)) {
        Ok(_) => sentinel_error_t::SENTINEL_OK,
        Err(e) => {
            set_error(format!("Failed to delete collection: {}", e));
            sentinel_error_t::SENTINEL_ERROR_IO_ERROR
        },
    }
}

/// List all collections in the store
/// Returns NULL on error, result is a JSON array string that must be freed
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_store_list_collections(store: *mut sentinel_store_t) -> *mut c_char {
    if store.is_null() {
        set_error("Store cannot be null");
        return ptr::null_mut();
    }

    let store_ref = unsafe { &*(store as *mut Store) };

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return ptr::null_mut();
        },
    };

    let collections = match rt.block_on(store_ref.list_collections()) {
        Ok(collections) => collections,
        Err(e) => {
            set_error(format!("Failed to list collections: {}", e));
            return ptr::null_mut();
        },
    };

    let json = match serde_json::to_string(&collections) {
        Ok(json) => json,
        Err(e) => {
            set_error(format!("Failed to serialize collections: {}", e));
            return ptr::null_mut();
        },
    };

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(e) => {
            set_error(format!("Failed to create C string: {}", e));
            ptr::null_mut()
        },
    }
}

/// Free a Sentinel collection
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_free(collection: *mut sentinel_collection_t) {
    if !collection.is_null() {
        unsafe {
            let _ = Box::from_raw(collection as *mut Collection);
        }
    }
}

/// Insert a document into a collection
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_insert(
    collection: *mut sentinel_collection_t,
    id: *const c_char,
    json_data: *const c_char,
) -> sentinel_error_t {
    if collection.is_null() || id.is_null() || json_data.is_null() {
        set_error("Collection, id, and data cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let collection_ref = unsafe { &*(collection as *mut Collection) };
    let id_str = match unsafe { CStr::from_ptr(id) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in document id: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let json_str = match unsafe { CStr::from_ptr(json_data) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in JSON data: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let data: Value = match serde_json::from_str(json_str) {
        Ok(data) => data,
        Err(e) => {
            set_error(format!("Invalid JSON: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_JSON_PARSE_ERROR;
        },
    };

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_RUNTIME_ERROR;
        },
    };

    match rt.block_on(collection_ref.insert(id_str, data)) {
        Ok(_) => sentinel_error_t::SENTINEL_OK,
        Err(e) => {
            set_error(format!("Failed to insert document: {}", e));
            sentinel_error_t::SENTINEL_ERROR_IO_ERROR
        },
    }
}

/// Get a document by ID
/// Returns NULL if not found or on error, check sentinel_get_last_error() for details
/// Result is a JSON string that must be freed
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_get(
    collection: *mut sentinel_collection_t,
    id: *const c_char,
) -> *mut c_char {
    if collection.is_null() || id.is_null() {
        set_error("Collection and id cannot be null");
        return ptr::null_mut();
    }

    let collection_ref = unsafe { &*(collection as *mut Collection) };
    let id_str = match unsafe { CStr::from_ptr(id) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in document id: {}", e));
            return ptr::null_mut();
        },
    };

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return ptr::null_mut();
        },
    };

    let document = match rt.block_on(collection_ref.get(id_str)) {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            set_error("Document not found");
            return ptr::null_mut();
        },
        Err(e) => {
            set_error(format!("Failed to get document: {}", e));
            return ptr::null_mut();
        },
    };

    let json = match serde_json::to_string(document.data()) {
        Ok(json) => json,
        Err(e) => {
            set_error(format!("Failed to serialize document: {}", e));
            return ptr::null_mut();
        },
    };

    match CString::new(json) {
        Ok(cstr) => cstr.into_raw(),
        Err(e) => {
            set_error(format!("Failed to create C string: {}", e));
            ptr::null_mut()
        },
    }
}

/// Delete a document by ID
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_delete(
    collection: *mut sentinel_collection_t,
    id: *const c_char,
) -> sentinel_error_t {
    if collection.is_null() || id.is_null() {
        set_error("Collection and id cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let collection_ref = unsafe { &*(collection as *mut Collection) };
    let id_str = match unsafe { CStr::from_ptr(id) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in document id: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_RUNTIME_ERROR;
        },
    };

    match rt.block_on(collection_ref.delete(id_str)) {
        Ok(_) => sentinel_error_t::SENTINEL_OK,
        Err(e) => {
            set_error(format!("Failed to delete document: {}", e));
            sentinel_error_t::SENTINEL_ERROR_IO_ERROR
        },
    }
}

/// Get the count of documents in the collection
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_count(
    collection: *mut sentinel_collection_t,
    count: *mut u32,
) -> sentinel_error_t {
    if collection.is_null() || count.is_null() {
        set_error("Collection and count pointer cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let collection_ref = unsafe { &*(collection as *mut Collection) };

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_RUNTIME_ERROR;
        },
    };

    match rt.block_on(collection_ref.count()) {
        Ok(c) => {
            unsafe { *count = c as u32 };
            sentinel_error_t::SENTINEL_OK
        },
        Err(e) => {
            set_error(format!("Failed to count documents: {}", e));
            sentinel_error_t::SENTINEL_ERROR_IO_ERROR
        },
    }
}

/// Update a document
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_update(
    collection: *mut sentinel_collection_t,
    id: *const c_char,
    json_data: *const c_char,
) -> sentinel_error_t {
    if collection.is_null() || id.is_null() || json_data.is_null() {
        set_error("Collection, id, and data cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let collection_ref = unsafe { &*(collection as *mut Collection) };
    let id_str = match unsafe { CStr::from_ptr(id) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in document id: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let json_str = match unsafe { CStr::from_ptr(json_data) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in JSON data: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let data: Value = match serde_json::from_str(json_str) {
        Ok(data) => data,
        Err(e) => {
            set_error(format!("Invalid JSON: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_JSON_PARSE_ERROR;
        },
    };

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_RUNTIME_ERROR;
        },
    };

    match rt.block_on(collection_ref.update(id_str, data)) {
        Ok(_) => sentinel_error_t::SENTINEL_OK,
        Err(e) => {
            set_error(format!("Failed to update document: {}", e));
            sentinel_error_t::SENTINEL_ERROR_IO_ERROR
        },
    }
}

/// Upsert a document (insert or update)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_upsert(
    collection: *mut sentinel_collection_t,
    id: *const c_char,
    json_data: *const c_char,
    was_insert: *mut bool,
) -> sentinel_error_t {
    if collection.is_null() || id.is_null() || json_data.is_null() || was_insert.is_null() {
        set_error("Collection, id, data, and was_insert pointer cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let collection_ref = unsafe { &*(collection as *mut Collection) };
    let id_str = match unsafe { CStr::from_ptr(id) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in document id: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let json_str = match unsafe { CStr::from_ptr(json_data) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in JSON data: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let data: Value = match serde_json::from_str(json_str) {
        Ok(data) => data,
        Err(e) => {
            set_error(format!("Invalid JSON: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_JSON_PARSE_ERROR;
        },
    };

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_RUNTIME_ERROR;
        },
    };

    match rt.block_on(collection_ref.upsert(id_str, data)) {
        Ok(was_ins) => {
            unsafe { *was_insert = was_ins };
            sentinel_error_t::SENTINEL_OK
        },
        Err(e) => {
            set_error(format!("Failed to upsert document: {}", e));
            sentinel_error_t::SENTINEL_ERROR_IO_ERROR
        },
    }
}

/// Create a new query with a simple filter
/// Returns NULL on error, check sentinel_get_last_error() for details
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_new_simple(
    field: *const c_char,
    value: *const c_char,
) -> *mut sentinel_query_t {
    if field.is_null() || value.is_null() {
        set_error("Field and value cannot be null");
        return ptr::null_mut();
    }

    let field_str = match unsafe { CStr::from_ptr(field) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in field: {}", e));
            return ptr::null_mut();
        },
    };

    let value_str = match unsafe { CStr::from_ptr(value) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in value: {}", e));
            return ptr::null_mut();
        },
    };

    let json_value: Value = match serde_json::from_str(value_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(format!("Invalid JSON value: {}", e));
            return ptr::null_mut();
        },
    };

    let filter = Filter::Equals(field_str, json_value);
    let query = Query {
        filters:    vec![filter],
        sort:       None,
        limit:      None,
        offset:     None,
        projection: None,
    };

    Box::into_raw(Box::new(query)) as *mut sentinel_query_t
}

/// Free a query
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_free(query: *mut sentinel_query_t) {
    if !query.is_null() {
        unsafe {
            let _ = Box::from_raw(query as *mut Query);
        }
    }
}

/// Execute a query synchronously
/// Returns JSON array of matching documents, NULL on error
/// Check sentinel_get_last_error() for details
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_query(
    collection: *mut sentinel_collection_t,
    query: *mut sentinel_query_t,
) -> *mut c_char {
    if collection.is_null() || query.is_null() {
        set_error("Collection and query cannot be null");
        return ptr::null_mut();
    }

    let collection_ref = unsafe { &*(collection as *mut Collection) };
    let query_ref = unsafe { &*(query as *mut Query) };

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return ptr::null_mut();
        },
    };

    let query_result = match rt.block_on(collection_ref.query(query_ref.clone())) {
        Ok(result) => result,
        Err(e) => {
            set_error(format!("Query execution failed: {}", e));
            return ptr::null_mut();
        },
    };

    // For simplicity, return just the total count for now
    // Full streaming implementation would be more complex
    let result_json = match serde_json::to_string(&query_result.total_count.unwrap_or(0)) {
        Ok(json) => json,
        Err(e) => {
            set_error(format!("Serialization error: {}", e));
            return ptr::null_mut();
        },
    };

    match CString::new(result_json) {
        Ok(cstr) => cstr.into_raw(),
        Err(e) => {
            set_error(format!("Failed to create C string: {}", e));
            ptr::null_mut()
        },
    }
}

/// Get the last error message as a C string
/// Returns NULL if no error occurred
/// The returned string must be freed with sentinel_string_free
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_get_last_error() -> *mut c_char {
    match get_error() {
        Some(err) => {
            match CString::new(err) {
                Ok(cstr) => cstr.into_raw(),
                Err(_) => ptr::null_mut(),
            }
        },
        None => ptr::null_mut(),
    }
}

/// Free a string returned by Sentinel functions
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}
