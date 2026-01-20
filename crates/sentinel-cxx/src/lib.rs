use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    ptr,
    sync::{Arc, Mutex},
};

use sentinel_dbms::{Collection, Filter, Query, Store};
use tokio::runtime::Runtime;
use serde_json::Value;
use once_cell::sync::Lazy;

static RUNTIME: Lazy<Mutex<Runtime>> =
    Lazy::new(|| Mutex::new(Runtime::new().expect("Failed to create Tokio runtime")));

thread_local! {
    static LAST_ERROR: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

fn set_error(err: impl std::fmt::Display) { LAST_ERROR.with(|e| *e.borrow_mut() = Some(err.to_string())); }

fn get_error() -> Option<String> { LAST_ERROR.with(|e| e.borrow().clone()) }

#[repr(C)]
pub struct sentinel_store_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct sentinel_collection_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct sentinel_document_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct sentinel_query_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct sentinel_task_t {
    _private: [u8; 0],
}

#[repr(C)]
pub enum sentinel_error_t {
    SENTINEL_OK                     = 0,
    SENTINEL_ERROR_NULL_POINTER     = 1,
    SENTINEL_ERROR_INVALID_ARGUMENT = 2,
    SENTINEL_ERROR_IO_ERROR         = 3,
    SENTINEL_ERROR_RUNTIME_ERROR    = 4,
    SENTINEL_ERROR_JSON_PARSE_ERROR = 5,
    SENTINEL_ERROR_NOT_FOUND        = 6,
    SENTINEL_ERROR_TASK_NOT_FOUND   = 7,
    SENTINEL_ERROR_TASK_PENDING     = 8,
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_store_free(store: *mut sentinel_store_t) {
    if !store.is_null() {
        unsafe {
            let _ = Box::from_raw(store as *mut Store);
        }
    }
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_free(collection: *mut sentinel_collection_t) {
    if !collection.is_null() {
        unsafe {
            let _ = Box::from_raw(collection as *mut Collection);
        }
    }
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_free(query: *mut sentinel_query_t) {
    if !query.is_null() {
        unsafe {
            let _ = Box::from_raw(query as *mut Query);
        }
    }
}

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_string_free(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }
}

type StoreCallback = Option<unsafe extern "C" fn(task_id: u64, result: *mut sentinel_store_t, user_data: *mut c_char)>;
type CollectionCallback =
    Option<unsafe extern "C" fn(task_id: u64, result: *mut sentinel_collection_t, user_data: *mut c_char)>;
type VoidCallback = Option<unsafe extern "C" fn(task_id: u64, user_data: *mut c_char)>;
type DocumentCallback = Option<unsafe extern "C" fn(task_id: u64, result: *mut c_char, user_data: *mut c_char)>;
type BoolCallback = Option<unsafe extern "C" fn(task_id: u64, result: bool, user_data: *mut c_char)>;
type CountCallback = Option<unsafe extern "C" fn(task_id: u64, result: u32, user_data: *mut c_char)>;
type ErrorCallback = Option<unsafe extern "C" fn(task_id: u64, error: *const c_char, user_data: *mut c_char)>;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_store_new_async(
    path: *const c_char,
    passphrase: *const c_char,
    callback: StoreCallback,
    error_callback: ErrorCallback,
    user_data: *mut c_char,
) -> u64 {
    if path.is_null() {
        set_error("Path cannot be null");
        return 0;
    }

    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in path: {}", e));
            return 0;
        },
    };

    let passphrase_str = if !passphrase.is_null() {
        match unsafe { CStr::from_ptr(passphrase) }.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(e) => {
                set_error(format!("Invalid UTF-8 in passphrase: {}", e));
                return 0;
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
            return 0;
        },
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let user_data_usize = user_data as usize;
    let callback_ptr = callback.map(|cb| cb as usize);
    let error_callback_ptr = error_callback.map(|cb| cb as usize);

    // Use synchronous Store::new with a temporary runtime
    let result = std::thread::spawn(move || {
        // Create a temporary runtime for this thread
        let rt = Runtime::new().expect("Failed to create temp runtime");
        rt.block_on(async { Store::new(&path_str, passphrase_str.as_deref()).await })
    })
    .join()
    .expect("Thread panicked");
    let _ = tx.send(result);

    // Handle result in a separate thread to call callbacks
    std::thread::spawn(move || {
        let user_data = user_data_usize as *mut c_char;
        if let Ok(result) = rx.recv() {
            match result {
                Ok(store) => {
                    let store_ptr = Box::into_raw(Box::new(store)) as *mut sentinel_store_t;
                    if let Some(cb_ptr) = callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, StoreCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, store_ptr, user_data) }; // task_id not used
                        }
                    }
                },
                Err(err) => {
                    let err_cstr = match CString::new(err.to_string()) {
                        Ok(cstr) => cstr,
                        Err(_) => return,
                    };
                    if let Some(cb_ptr) = error_callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, ErrorCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, err_cstr.as_ptr(), user_data) }; // task_id not used
                        }
                    }
                },
            }
        }
    });

    1 // Return task ID
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_new_async(
    store: *mut sentinel_store_t,
    name: *const c_char,
    callback: CollectionCallback,
    error_callback: ErrorCallback,
    user_data: *mut c_char,
) -> u64 {
    unsafe { sentinel_store_collection_async(store, name, callback, error_callback, user_data) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_store_collection_async(
    store: *mut sentinel_store_t,
    name: *const c_char,
    callback: CollectionCallback,
    error_callback: ErrorCallback,
    user_data: *mut c_char,
) -> u64 {
    if store.is_null() || name.is_null() {
        set_error("Store and name cannot be null");
        return 0;
    }

    if callback.is_none() || error_callback.is_none() {
        set_error("Callbacks cannot be null for async operations");
        return 0;
    }

    // SAFETY: The store pointer must be valid for the lifetime of the async operation.
    // We wrap it in Arc to ensure the Store outlives the spawned task.
    let store_arc = unsafe {
        let store_box = Box::from_raw(store as *mut Store);
        Arc::new(*store_box)
    };

    let store_ref = Arc::clone(&store_arc);
    let name_str = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in collection name: {}", e));
            return 0;
        },
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let user_data_usize = user_data as usize;
    let callback_ptr = callback.map(|cb| cb as usize);
    let error_callback_ptr = error_callback.map(|cb| cb as usize);

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return 0;
        },
    };

    rt.spawn(async move {
        let result = store_ref.collection(&name_str).await;
        let _ = tx.send(result);
    });

    // Handle result in a separate thread to call callbacks
    std::thread::spawn(move || {
        let user_data = user_data_usize as *mut c_char;
        if let Ok(result) = rx.recv() {
            match result {
                Ok(collection) => {
                    let coll_ptr = Box::into_raw(Box::new(collection)) as *mut sentinel_collection_t;
                    if let Some(cb_ptr) = callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, CollectionCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, coll_ptr, user_data) }; // task_id not used
                        }
                    }
                },
                Err(err) => {
                    let err_cstr = match CString::new(err.to_string()) {
                        Ok(cstr) => cstr,
                        Err(_) => return,
                    };
                    if let Some(cb_ptr) = error_callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, ErrorCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, err_cstr.as_ptr(), user_data) }; // task_id not used
                        }
                    }
                },
            }
        }
    });

    0 // Return 0 since we don't use task IDs
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_insert_async(
    collection: *mut sentinel_collection_t,
    id: *const c_char,
    json_data: *const c_char,
    callback: VoidCallback,
    error_callback: ErrorCallback,
    user_data: *mut c_char,
) -> u64 {
    if collection.is_null() || id.is_null() || json_data.is_null() {
        set_error("Collection, id, and data cannot be null");
        return 0;
    }

    if callback.is_none() || error_callback.is_none() {
        set_error("Callbacks cannot be null for async operations");
        return 0;
    }

    // SAFETY: The collection pointer must be valid for the lifetime of the async operation.
    // We wrap it in Arc to ensure the Collection outlives the spawned task.
    let collection_arc = unsafe {
        let collection_box = Box::from_raw(collection as *mut Collection);
        Arc::new(*collection_box)
    };

    let collection_ref = Arc::clone(&collection_arc);
    let id_str = match unsafe { CStr::from_ptr(id) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in document id: {}", e));
            return 0;
        },
    };

    let json_str = match unsafe { CStr::from_ptr(json_data) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in JSON data: {}", e));
            return 0;
        },
    };

    let data: Value = match serde_json::from_str(json_str) {
        Ok(data) => data,
        Err(e) => {
            set_error(format!("Invalid JSON: {}", e));
            return 0;
        },
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let user_data_usize = user_data as usize;
    let callback_ptr = callback.map(|cb| cb as usize);
    let error_callback_ptr = error_callback.map(|cb| cb as usize);

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return 0;
        },
    };

    rt.spawn(async move {
        let result = collection_ref.insert(&id_str, data).await;
        let _ = tx.send(result);
    });

    // Handle result in a separate thread to call callbacks
    std::thread::spawn(move || {
        let user_data = user_data_usize as *mut c_char;
        if let Ok(result) = rx.recv() {
            match result {
                Ok(_) => {
                    if let Some(cb_ptr) = callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, VoidCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, user_data) }; // task_id not used
                        }
                    }
                },
                Err(err) => {
                    let err_cstr = match CString::new(err.to_string()) {
                        Ok(cstr) => cstr,
                        Err(_) => return,
                    };
                    if let Some(cb_ptr) = error_callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, ErrorCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, err_cstr.as_ptr(), user_data) }; // task_id not used
                        }
                    }
                },
            }
        }
    });

    0 // Return 0 since we don't use task IDs
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_get_async(
    collection: *mut sentinel_collection_t,
    id: *const c_char,
    callback: DocumentCallback,
    error_callback: ErrorCallback,
    user_data: *mut c_char,
) -> u64 {
    if collection.is_null() || id.is_null() {
        set_error("Collection and id cannot be null");
        return 0;
    }

    if callback.is_none() || error_callback.is_none() {
        set_error("Callbacks cannot be null for async operations");
        return 0;
    }

    // SAFETY: The collection pointer must be valid for the lifetime of the async operation.
    // We wrap it in Arc to ensure the Collection outlives the spawned task.
    let collection_arc = unsafe {
        let collection_box = Box::from_raw(collection as *mut Collection);
        Arc::new(*collection_box)
    };

    let collection_ref = Arc::clone(&collection_arc);
    let id_str = match unsafe { CStr::from_ptr(id) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in document id: {}", e));
            return 0;
        },
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let user_data_usize = user_data as usize;
    let callback_ptr = callback.map(|cb| cb as usize);
    let error_callback_ptr = error_callback.map(|cb| cb as usize);

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return 0;
        },
    };

    rt.spawn(async move {
        let result = collection_ref.get(&id_str).await;
        let _ = tx.send(result);
    });

    // Handle result in a separate thread to call callbacks
    std::thread::spawn(move || {
        let user_data = user_data_usize as *mut c_char;
        if let Ok(result) = rx.recv() {
            match result {
                Ok(Some(doc)) => {
                    let json_result = match serde_json::to_string(doc.data()) {
                        Ok(json) => json,
                        Err(e) => {
                            let err_cstr = match CString::new(format!("Serialization error: {}", e)) {
                                Ok(cstr) => cstr,
                                Err(_) => return,
                            };
                            if let Some(cb_ptr) = error_callback_ptr {
                                let cb = unsafe { std::mem::transmute::<usize, ErrorCallback>(cb_ptr) };
                                if let Some(cb) = cb {
                                    unsafe { cb(0, err_cstr.as_ptr(), user_data) };
                                }
                            }
                            return;
                        },
                    };

                    let json_cstr = match CString::new(json_result) {
                        Ok(cstr) => cstr,
                        Err(_) => return,
                    };

                    if let Some(cb_ptr) = callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, DocumentCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, json_cstr.into_raw(), user_data) };
                        }
                    }
                },
                Ok(None) => {
                    // Document not found
                    let err_cstr = match CString::new("Document not found") {
                        Ok(cstr) => cstr,
                        Err(_) => return,
                    };
                    if let Some(cb_ptr) = error_callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, ErrorCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, err_cstr.as_ptr(), user_data) };
                        }
                    }
                },
                Err(err) => {
                    let err_cstr = match CString::new(err.to_string()) {
                        Ok(cstr) => cstr,
                        Err(_) => return,
                    };
                    if let Some(cb_ptr) = error_callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, ErrorCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, err_cstr.as_ptr(), user_data) };
                        }
                    }
                },
            }
        }
    });

    0 // Return 0 since we don't use task IDs
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_update_async(
    collection: *mut sentinel_collection_t,
    id: *const c_char,
    json_data: *const c_char,
    callback: VoidCallback,
    error_callback: ErrorCallback,
    user_data: *mut c_char,
) -> u64 {
    if collection.is_null() || id.is_null() || json_data.is_null() {
        set_error("Collection, id, and data cannot be null");
        return 0;
    }

    if callback.is_none() || error_callback.is_none() {
        set_error("Callbacks cannot be null for async operations");
        return 0;
    }

    // SAFETY: The collection pointer must be valid for the lifetime of the async operation.
    // We wrap it in Arc to ensure the Collection outlives the spawned task.
    let collection_arc = unsafe {
        let collection_box = Box::from_raw(collection as *mut Collection);
        Arc::new(*collection_box)
    };

    let collection_ref = Arc::clone(&collection_arc);
    let id_str = match unsafe { CStr::from_ptr(id) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in document id: {}", e));
            return 0;
        },
    };

    let json_str = match unsafe { CStr::from_ptr(json_data) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in JSON data: {}", e));
            return 0;
        },
    };

    let data: Value = match serde_json::from_str(json_str) {
        Ok(data) => data,
        Err(e) => {
            set_error(format!("Invalid JSON: {}", e));
            return 0;
        },
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let user_data_usize = user_data as usize;
    let callback_ptr = callback.map(|cb| cb as usize);
    let error_callback_ptr = error_callback.map(|cb| cb as usize);

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return 0;
        },
    };

    rt.spawn(async move {
        let result = collection_ref.update(&id_str, data).await;
        let _ = tx.send(result);
    });

    // Handle result in a separate thread to call callbacks
    std::thread::spawn(move || {
        let user_data = user_data_usize as *mut c_char;
        if let Ok(result) = rx.recv() {
            match result {
                Ok(_) => {
                    if let Some(cb_ptr) = callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, VoidCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, user_data) };
                        }
                    }
                },
                Err(err) => {
                    let err_cstr = match CString::new(err.to_string()) {
                        Ok(cstr) => cstr,
                        Err(_) => return,
                    };
                    if let Some(cb_ptr) = error_callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, ErrorCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, err_cstr.as_ptr(), user_data) };
                        }
                    }
                },
            }
        }
    });

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_upsert_async(
    collection: *mut sentinel_collection_t,
    id: *const c_char,
    json_data: *const c_char,
    callback: BoolCallback,
    error_callback: ErrorCallback,
    user_data: *mut c_char,
) -> u64 {
    if collection.is_null() || id.is_null() || json_data.is_null() {
        set_error("Collection, id, and data cannot be null");
        return 0;
    }

    if callback.is_none() || error_callback.is_none() {
        set_error("Callbacks cannot be null for async operations");
        return 0;
    }

    let collection_ref = unsafe { &*(collection as *mut Collection) };
    let id_str = match unsafe { CStr::from_ptr(id) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in document id: {}", e));
            return 0;
        },
    };

    let json_str = match unsafe { CStr::from_ptr(json_data) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in JSON data: {}", e));
            return 0;
        },
    };

    let data: Value = match serde_json::from_str(json_str) {
        Ok(data) => data,
        Err(e) => {
            set_error(format!("Invalid JSON: {}", e));
            return 0;
        },
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let user_data_usize = user_data as usize;
    let callback_ptr = callback.map(|cb| cb as usize);
    let error_callback_ptr = error_callback.map(|cb| cb as usize);

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return 0;
        },
    };

    rt.spawn(async move {
        let result = collection_ref.upsert(&id_str, data).await;
        let _ = tx.send(result);
    });

    // Handle result in a separate thread to call callbacks
    std::thread::spawn(move || {
        let user_data = user_data_usize as *mut c_char;
        if let Ok(result) = rx.recv() {
            match result {
                Ok(was_insert) => {
                    if let Some(cb_ptr) = callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, BoolCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, was_insert, user_data) };
                        }
                    }
                },
                Err(err) => {
                    let err_cstr = match CString::new(err.to_string()) {
                        Ok(cstr) => cstr,
                        Err(_) => return,
                    };
                    if let Some(cb_ptr) = error_callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, ErrorCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, err_cstr.as_ptr(), user_data) };
                        }
                    }
                },
            }
        }
    });

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_delete_async(
    collection: *mut sentinel_collection_t,
    id: *const c_char,
    callback: VoidCallback,
    error_callback: ErrorCallback,
    user_data: *mut c_char,
) -> u64 {
    if collection.is_null() || id.is_null() {
        set_error("Collection and id cannot be null");
        return 0;
    }

    if callback.is_none() || error_callback.is_none() {
        set_error("Callbacks cannot be null for async operations");
        return 0;
    }

    let collection_ref = unsafe { &*(collection as *mut Collection) };
    let id_str = match unsafe { CStr::from_ptr(id) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in document id: {}", e));
            return 0;
        },
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let user_data_usize = user_data as usize;
    let callback_ptr = callback.map(|cb| cb as usize);
    let error_callback_ptr = error_callback.map(|cb| cb as usize);

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return 0;
        },
    };

    rt.spawn(async move {
        let result = collection_ref.delete(&id_str).await;
        let _ = tx.send(result);
    });

    // Handle result in a separate thread to call callbacks
    std::thread::spawn(move || {
        let user_data = user_data_usize as *mut c_char;
        if let Ok(result) = rx.recv() {
            match result {
                Ok(_) => {
                    if let Some(cb_ptr) = callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, VoidCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, user_data) };
                        }
                    }
                },
                Err(err) => {
                    let err_cstr = match CString::new(err.to_string()) {
                        Ok(cstr) => cstr,
                        Err(_) => return,
                    };
                    if let Some(cb_ptr) = error_callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, ErrorCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, err_cstr.as_ptr(), user_data) };
                        }
                    }
                },
            }
        }
    });

    0
}

/// Get the count of documents asynchronously
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_count_async(
    collection: *mut sentinel_collection_t,
    callback: CountCallback,
    error_callback: ErrorCallback,
    user_data: *mut c_char,
) -> u64 {
    if collection.is_null() {
        set_error("Collection cannot be null");
        return 0;
    }

    if callback.is_none() || error_callback.is_none() {
        set_error("Callbacks cannot be null for async operations");
        return 0;
    }

    let collection_ref = unsafe { &*(collection as *mut Collection) };

    let (tx, rx) = std::sync::mpsc::channel();
    let user_data_usize = user_data as usize;
    let callback_ptr = callback.map(|cb| cb as usize);
    let error_callback_ptr = error_callback.map(|cb| cb as usize);

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return 0;
        },
    };

    rt.spawn(async move {
        let result = collection_ref.count().await;
        let _ = tx.send(result);
    });

    // Handle result in a separate thread to call callbacks
    std::thread::spawn(move || {
        let user_data = user_data_usize as *mut c_char;
        if let Ok(result) = rx.recv() {
            match result {
                Ok(count) => {
                    if let Some(cb_ptr) = callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, CountCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, count as u32, user_data) };
                        }
                    }
                },
                Err(err) => {
                    let err_cstr = match CString::new(err.to_string()) {
                        Ok(cstr) => cstr,
                        Err(_) => return,
                    };
                    if let Some(cb_ptr) = error_callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, ErrorCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, err_cstr.as_ptr(), user_data) };
                        }
                    }
                },
            }
        }
    });

    0
}

/// Execute a query asynchronously
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_query_async(
    collection: *mut sentinel_collection_t,
    query: *mut sentinel_query_t,
    callback: DocumentCallback,
    error_callback: ErrorCallback,
    user_data: *mut c_char,
) -> u64 {
    if collection.is_null() || query.is_null() {
        set_error("Collection and query cannot be null");
        return 0;
    }

    if callback.is_none() || error_callback.is_none() {
        set_error("Callbacks cannot be null for async operations");
        return 0;
    }

    let collection_ref = unsafe { &*(collection as *mut Collection) };
    let query_ref = unsafe { &*(query as *mut Query) };

    let (tx, rx) = std::sync::mpsc::channel();
    let user_data_usize = user_data as usize;
    let callback_ptr = callback.map(|cb| cb as usize);
    let error_callback_ptr = error_callback.map(|cb| cb as usize);

    let rt = match RUNTIME.lock() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(format!("Failed to acquire runtime lock: {}", e));
            return 0;
        },
    };

    let query_clone = query_ref.clone();
    rt.spawn(async move {
        let result = collection_ref.query(query_clone).await;
        match result {
            Ok(query_result) => {
                // For now, return total count as JSON string
                let count = query_result.total_count.unwrap_or(0);
                let json_str = serde_json::to_string(&count).unwrap_or_else(|_| "0".to_string());
                let _ = tx.send(Ok(json_str));
            },
            Err(e) => {
                let _ = tx.send(Err(e));
            },
        }
    });

    // Handle result in a separate thread to call callbacks
    std::thread::spawn(move || {
        let user_data = user_data_usize as *mut c_char;
        if let Ok(result) = rx.recv() {
            match result {
                Ok(json_str) => {
                    let cstr = match CString::new(json_str) {
                        Ok(cstr) => cstr,
                        Err(_) => return,
                    };

                    if let Some(cb_ptr) = callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, DocumentCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, cstr.into_raw(), user_data) };
                        }
                    }
                },
                Err(err) => {
                    let err_cstr = match CString::new(err.to_string()) {
                        Ok(cstr) => cstr,
                        Err(_) => return,
                    };
                    if let Some(cb_ptr) = error_callback_ptr {
                        let cb = unsafe { std::mem::transmute::<usize, ErrorCallback>(cb_ptr) };
                        if let Some(cb) = cb {
                            unsafe { cb(0, err_cstr.as_ptr(), user_data) };
                        }
                    }
                },
            }
        }
    });

    0 // Return task ID
}

/// Combine two queries with OR logic
/// Creates a new query that matches either the left OR right query
///
/// When combining queries with multiple filters (which are internally ANDed),
/// this function properly ORs the two filter groups together rather than
/// flattening and OR-ing every individual filter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_or(
    left: *mut sentinel_query_t,
    right: *mut sentinel_query_t,
) -> *mut sentinel_query_t {
    if left.is_null() || right.is_null() {
        set_error("Left and right queries cannot be null");
        return ptr::null_mut();
    }

    let left_query = unsafe { &*(left as *mut Query) };
    let right_query = unsafe { &*(right as *mut Query) };

    // Create a new query with combined filters using OR
    let mut combined_filters = Vec::new();

    // Handle the case where either query has multiple filters
    // Each query's filters should be treated as an AND group
    let left_has_multiple = left_query.filters.len() > 1;
    let right_has_multiple = right_query.filters.len() > 1;

    if left_query.filters.is_empty() && right_query.filters.is_empty() {
        // Both queries have no filters - return empty result (matches nothing)
        // This is correct since an empty OR would match nothing
    }
    else if left_query.filters.is_empty() {
        // Left has no filters, return right as-is
        combined_filters.extend(right_query.filters.clone());
    }
    else if right_query.filters.is_empty() {
        // Right has no filters, return left as-is
        combined_filters.extend(left_query.filters.clone());
    }
    else if left_query.filters.len() == 1 && right_query.filters.len() == 1 {
        // Simple case: both queries have single filters
        // OR the two individual filters
        let left_filter = left_query.filters[0].clone();
        let right_filter = right_query.filters[0].clone();
        combined_filters.push(sentinel_dbms::Filter::Or(
            Box::new(left_filter),
            Box::new(right_filter),
        ));
    }
    else if left_has_multiple && right_has_multiple {
        // Both have multiple filters - create AND groups and OR them
        let mut left_and = left_query.filters[0].clone();
        for filter in &left_query.filters[1 ..] {
            left_and = sentinel_dbms::Filter::And(Box::new(left_and), Box::new(filter.clone()));
        }

        let mut right_and = right_query.filters[0].clone();
        for filter in &right_query.filters[1 ..] {
            right_and = sentinel_dbms::Filter::And(Box::new(right_and), Box::new(filter.clone()));
        }

        combined_filters.push(sentinel_dbms::Filter::Or(
            Box::new(left_and),
            Box::new(right_and),
        ));
    }
    else if left_has_multiple {
        // Left has multiple filters (AND group), right has single filter
        let mut left_and = left_query.filters[0].clone();
        for filter in &left_query.filters[1 ..] {
            left_and = sentinel_dbms::Filter::And(Box::new(left_and), Box::new(filter.clone()));
        }

        let right_filter = right_query.filters[0].clone();
        combined_filters.push(sentinel_dbms::Filter::Or(
            Box::new(left_and),
            Box::new(right_filter),
        ));
    }
    else {
        // Right has multiple filters (AND group), left has single filter
        let mut right_and = right_query.filters[0].clone();
        for filter in &right_query.filters[1 ..] {
            right_and = sentinel_dbms::Filter::And(Box::new(right_and), Box::new(filter.clone()));
        }

        let left_filter = left_query.filters[0].clone();
        combined_filters.push(sentinel_dbms::Filter::Or(
            Box::new(left_filter),
            Box::new(right_and),
        ));
    }

    let new_query = Query {
        filters:    combined_filters,
        sort:       None, // Could potentially merge sort from both queries
        limit:      None, // Could potentially merge limit from both queries
        offset:     None, // Could potentially merge offset from both queries
        projection: None,
    };

    let boxed_query = Box::new(new_query);
    Box::into_raw(boxed_query) as *mut sentinel_query_t
}

/// Combine two queries with AND logic
/// Creates a new query that matches both the left AND right query
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_and(
    left: *mut sentinel_query_t,
    right: *mut sentinel_query_t,
) -> *mut sentinel_query_t {
    if left.is_null() || right.is_null() {
        set_error("Left and right queries cannot be null");
        return ptr::null_mut();
    }

    let left_query = unsafe { &*(left as *mut Query) };
    let right_query = unsafe { &*(right as *mut Query) };

    // Create a new query with combined filters using AND
    let mut combined_filters = Vec::new();

    // Combine all filters from both queries
    let mut all_filters = left_query.filters.clone();
    all_filters.extend(right_query.filters.clone());

    if all_filters.len() >= 2 {
        // Combine all filters with AND in a tree structure
        let mut combined = all_filters[0].clone();
        for filter in &all_filters[1 ..] {
            combined = sentinel_dbms::Filter::And(Box::new(combined), Box::new(filter.clone()));
        }
        combined_filters.push(combined);
    }
    else if all_filters.len() == 1 {
        combined_filters.push(all_filters[0].clone());
    }

    let new_query = Query {
        filters:    combined_filters,
        sort:       None, // Could potentially merge sort from both queries
        limit:      None, // Could potentially merge limit from both queries
        offset:     None, // Could potentially merge offset from both queries
        projection: None,
    };

    let boxed_query = Box::new(new_query);
    Box::into_raw(boxed_query) as *mut sentinel_query_t
}

/// Create a new query builder
/// Returns NULL on error
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_new() -> *mut sentinel_query_t {
    use sentinel_dbms::QueryBuilder;
    let builder = QueryBuilder::new();
    let query = builder.build();
    Box::into_raw(Box::new(query)) as *mut sentinel_query_t
}

/// Add an equality filter to a query
/// Returns 0 on success, error code on failure
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_filter_equals(
    query: *mut sentinel_query_t,
    field: *const c_char,
    json_value: *const c_char,
) -> sentinel_error_t {
    if query.is_null() || field.is_null() || json_value.is_null() {
        set_error("Query, field, and value cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };
    let field_str = match unsafe { CStr::from_ptr(field) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in field: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let value_str = match unsafe { CStr::from_ptr(json_value) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in value: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let value: Value = match serde_json::from_str(value_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(format!("Invalid JSON value: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_JSON_PARSE_ERROR;
        },
    };

    // Convert to QueryBuilder, add filter, convert back

    query_ref
        .filters
        .push(sentinel_dbms::Filter::Equals(field_str, value));

    sentinel_error_t::SENTINEL_OK
}

/// Add a greater than filter to a query
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_filter_greater_than(
    query: *mut sentinel_query_t,
    field: *const c_char,
    json_value: *const c_char,
) -> sentinel_error_t {
    if query.is_null() || field.is_null() || json_value.is_null() {
        set_error("Query, field, and value cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };
    let field_str = match unsafe { CStr::from_ptr(field) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in field: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let value_str = match unsafe { CStr::from_ptr(json_value) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in value: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let value: Value = match serde_json::from_str(value_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(format!("Invalid JSON value: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_JSON_PARSE_ERROR;
        },
    };

    query_ref
        .filters
        .push(sentinel_dbms::Filter::GreaterThan(field_str, value));

    sentinel_error_t::SENTINEL_OK
}

/// Add a less than filter to a query
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_filter_less_than(
    query: *mut sentinel_query_t,
    field: *const c_char,
    json_value: *const c_char,
) -> sentinel_error_t {
    if query.is_null() || field.is_null() || json_value.is_null() {
        set_error("Query, field, and value cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };
    let field_str = match unsafe { CStr::from_ptr(field) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in field: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let value_str = match unsafe { CStr::from_ptr(json_value) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in value: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let value: Value = match serde_json::from_str(value_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(format!("Invalid JSON value: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_JSON_PARSE_ERROR;
        },
    };

    query_ref
        .filters
        .push(sentinel_dbms::Filter::LessThan(field_str, value));

    sentinel_error_t::SENTINEL_OK
}

/// Add a contains filter to a query (for string fields)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_filter_contains(
    query: *mut sentinel_query_t,
    field: *const c_char,
    substring: *const c_char,
) -> sentinel_error_t {
    if query.is_null() || field.is_null() || substring.is_null() {
        set_error("Query, field, and substring cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };
    let field_str = match unsafe { CStr::from_ptr(field) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in field: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let substring_str = match unsafe { CStr::from_ptr(substring) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in substring: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    query_ref
        .filters
        .push(sentinel_dbms::Filter::Contains(field_str, substring_str));

    sentinel_error_t::SENTINEL_OK
}

/// Add a greater or equal filter to a query
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_filter_greater_or_equal(
    query: *mut sentinel_query_t,
    field: *const c_char,
    json_value: *const c_char,
) -> sentinel_error_t {
    if query.is_null() || field.is_null() || json_value.is_null() {
        set_error("Query, field, and value cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };
    let field_str = match unsafe { CStr::from_ptr(field) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in field: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let value_str = match unsafe { CStr::from_ptr(json_value) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in value: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let value: Value = match serde_json::from_str(value_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(format!("Invalid JSON value: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_JSON_PARSE_ERROR;
        },
    };

    query_ref
        .filters
        .push(sentinel_dbms::Filter::GreaterOrEqual(field_str, value));

    sentinel_error_t::SENTINEL_OK
}

/// Add a less or equal filter to a query
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_filter_less_or_equal(
    query: *mut sentinel_query_t,
    field: *const c_char,
    json_value: *const c_char,
) -> sentinel_error_t {
    if query.is_null() || field.is_null() || json_value.is_null() {
        set_error("Query, field, and value cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };
    let field_str = match unsafe { CStr::from_ptr(field) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in field: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let value_str = match unsafe { CStr::from_ptr(json_value) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in value: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let value: Value = match serde_json::from_str(value_str) {
        Ok(v) => v,
        Err(e) => {
            set_error(format!("Invalid JSON value: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_JSON_PARSE_ERROR;
        },
    };

    query_ref
        .filters
        .push(sentinel_dbms::Filter::LessOrEqual(field_str, value));

    sentinel_error_t::SENTINEL_OK
}

/// Add a starts with filter to a query (for string fields)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_filter_starts_with(
    query: *mut sentinel_query_t,
    field: *const c_char,
    prefix: *const c_char,
) -> sentinel_error_t {
    if query.is_null() || field.is_null() || prefix.is_null() {
        set_error("Query, field, and prefix cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };
    let field_str = match unsafe { CStr::from_ptr(field) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in field: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let prefix_str = match unsafe { CStr::from_ptr(prefix) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in prefix: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    query_ref
        .filters
        .push(sentinel_dbms::Filter::StartsWith(field_str, prefix_str));

    sentinel_error_t::SENTINEL_OK
}

/// Add an ends with filter to a query (for string fields)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_filter_ends_with(
    query: *mut sentinel_query_t,
    field: *const c_char,
    suffix: *const c_char,
) -> sentinel_error_t {
    if query.is_null() || field.is_null() || suffix.is_null() {
        set_error("Query, field, and suffix cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };
    let field_str = match unsafe { CStr::from_ptr(field) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in field: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let suffix_str = match unsafe { CStr::from_ptr(suffix) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in suffix: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    query_ref
        .filters
        .push(sentinel_dbms::Filter::EndsWith(field_str, suffix_str));

    sentinel_error_t::SENTINEL_OK
}

/// Add an in filter to a query (field value in array)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_filter_in(
    query: *mut sentinel_query_t,
    field: *const c_char,
    json_array: *const c_char,
) -> sentinel_error_t {
    if query.is_null() || field.is_null() || json_array.is_null() {
        set_error("Query, field, and array cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };
    let field_str = match unsafe { CStr::from_ptr(field) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in field: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let array_str = match unsafe { CStr::from_ptr(json_array) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("Invalid UTF-8 in array: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let array: Vec<Value> = match serde_json::from_str(array_str) {
        Ok(Value::Array(arr)) => arr,
        Ok(_) => {
            set_error("Expected JSON array");
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
        Err(e) => {
            set_error(format!("Invalid JSON array: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_JSON_PARSE_ERROR;
        },
    };

    query_ref
        .filters
        .push(sentinel_dbms::Filter::In(field_str, array));

    sentinel_error_t::SENTINEL_OK
}

/// Add an exists filter to a query (field exists or doesn't exist)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_filter_exists(
    query: *mut sentinel_query_t,
    field: *const c_char,
    should_exist: u32,
) -> sentinel_error_t {
    if query.is_null() || field.is_null() {
        set_error("Query and field cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };
    let field_str = match unsafe { CStr::from_ptr(field) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in field: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let exists = should_exist != 0;

    query_ref
        .filters
        .push(sentinel_dbms::Filter::Exists(field_str, exists));

    sentinel_error_t::SENTINEL_OK
}

/// Set sorting for a query
/// order: 0 = ascending, 1 = descending
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_sort(
    query: *mut sentinel_query_t,
    field: *const c_char,
    order: u32,
) -> sentinel_error_t {
    if query.is_null() || field.is_null() {
        set_error("Query and field cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };
    let field_str = match unsafe { CStr::from_ptr(field) }.to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            set_error(format!("Invalid UTF-8 in field: {}", e));
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    let sort_order = match order {
        0 => sentinel_dbms::SortOrder::Ascending,
        1 => sentinel_dbms::SortOrder::Descending,
        _ => {
            set_error("Invalid sort order: use 0 for ascending, 1 for descending");
            return sentinel_error_t::SENTINEL_ERROR_INVALID_ARGUMENT;
        },
    };

    query_ref.sort = Some((field_str, sort_order));

    sentinel_error_t::SENTINEL_OK
}

/// Set limit for a query
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_limit(query: *mut sentinel_query_t, limit: u32) -> sentinel_error_t {
    if query.is_null() {
        set_error("Query cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };

    query_ref.limit = Some(limit as usize);

    sentinel_error_t::SENTINEL_OK
}

/// Set offset for a query (for pagination)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_query_builder_offset(query: *mut sentinel_query_t, offset: u32) -> sentinel_error_t {
    if query.is_null() {
        set_error("Query cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let query_ref = unsafe { &mut *(query as *mut Query) };

    query_ref.offset = Some(offset as usize);

    sentinel_error_t::SENTINEL_OK
}
