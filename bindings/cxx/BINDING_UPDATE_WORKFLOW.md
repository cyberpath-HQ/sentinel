# Cyberpath Sentinel C/C++ Binding Update Workflow

This document outlines the complete process for updating C/C++ bindings when new APIs are added to the core Rust
Sentinel library.

## 📋 Table of Contents

- [Overview](#overview)
- [When to Update Bindings](#when-to-update-bindings)
- [Complete Workflow](#complete-workflow)
- [Tools and Scripts](#tools-and-scripts)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## 🎯 Overview

Cyberpath Sentinel uses a layered binding architecture:

```
Rust API (sentinel crate)
    ↓ [Manual C Wrappers]
C API (sentinel-cxx crate)
    ↓ [Auto-generated Headers]
C/C++ Headers (cbindgen)
    ↓ [CMake Build System]
C/C++ Applications
```

**Key Points:**

- **C wrapper functions are manually written** in `crates/sentinel-cxx/src/lib.rs`
- **C headers are auto-generated** via `cbindgen` from the wrapper functions
- **Build system is automated** via CMake + Cargo integration

## 🔍 When to Update Bindings

Update bindings when you add, remove, or modify:

- ✅ **New public functions** in the `sentinel` crate
- ✅ **New structs/enums** used in public APIs
- ✅ **Async methods** requiring callback-based C APIs
- ✅ **Error types** needing C-compatible representations
- ❌ **Private implementation details** (not exposed to C)

## 🚀 Complete Workflow

### Phase 1: Add Rust API

1. **Implement the new functionality** in the core `sentinel` crate:

   ```rust
   // crates/sentinel/src/collection.rs
   impl Collection {
       pub async fn enable_wal(&self, config: WalConfig) -> Result<()> {
           // Implementation
       }
   }
   ```

2. **Update public exports** in `crates/sentinel/src/lib.rs`:
   ```rust
   pub use collection::WalConfig;
   ```

### Phase 2: Detect API Changes

3. **Run the API synchronization monitor**:

   ```bash
   cd bindings
   python3 sync_api.py check
   ```

   This will:
   - Detect new public APIs
   - Generate a change report
   - Save `bindings/api_changes.md`

4. **Review the change report**:
   - Check for breaking changes
   - Verify all expected APIs are detected
   - Note any missing items

### Phase 3: Implement C Wrappers

5. **Add C-compatible wrapper functions** in `crates/sentinel-cxx/src/lib.rs`:

   ```rust
   // For async functions, create callback-based wrappers
   #[unsafe(no_mangle)]
   pub unsafe extern "C" fn sentinel_collection_enable_wal_async(
       collection: *mut sentinel_collection_t,
       config_json: *const c_char,
       callback: VoidCallback,
       error_callback: ErrorCallback,
       user_data: *mut c_char,
   ) -> u64 {
       // Validate inputs
       if collection.is_null() || config_json.is_null() {
           set_error("Collection and config cannot be null");
           return 0;
       }

       if callback.is_none() || error_callback.is_none() {
           set_error("Callbacks cannot be null for async operations");
           return 0;
       }

       // Convert C types to Rust types
       let collection_ref = unsafe { &*(collection as *mut Collection) };
       let config_str = unsafe { CStr::from_ptr(config_json) }
           .to_str()
           .unwrap_or("{}");

       let config: WalConfig = serde_json::from_str(config_str)
           .unwrap_or_default();

       // Spawn async task
       let rt = match RUNTIME.lock() {
           Ok(rt) => rt,
           Err(e) => {
               set_error(format!("Failed to acquire runtime: {}", e));
               return 0;
           },
       };

       // Handle the async result
       rt.spawn(async move {
           let result = collection_ref.enable_wal(config).await;

           match result {
               Ok(()) => {
                   // Call success callback
                   if let Some(cb) = callback {
                       unsafe { cb(0, user_data) };
                   }
               },
               Err(e) => {
                   // Call error callback
                   let error_msg = format!("{}", e);
                   let c_error = match CString::new(error_msg) {
                       Ok(s) => s,
                       Err(_) => return,
                   };
                   if let Some(cb) = error_callback {
                       unsafe { cb(0, c_error.as_ptr(), user_data) };
                   }
               }
           }
       });

       0 // Return task ID (0 for fire-and-forget)
   }
   ```

### Phase 4: Auto-Generate Headers

6. **Regenerate C headers automatically**:

   ```bash
   python3 bindings/sync_api.py update
   ```

   This will:
   - Build the `sentinel-cxx` crate
   - Run `cbindgen` to generate headers
   - Copy headers to `bindings/cxx/include/sentinel/`

### Phase 5: Build and Test

7. **Build the C/C++ bindings**:

   ```bash
   cd bindings/cxx/build
   make -j$(nproc)
   ```

8. **Test the new functionality**:

   ```bash
   # Create a test program
   cat > test_wal.c << 'EOF'
   #include <sentinel/sentinel-cxx.h>
   #include <stdio.h>

   void on_success(uint64_t task_id, void* user_data) {
       printf("✓ WAL enabled successfully\n");
   }

   void on_error(uint64_t task_id, const char* error, void* user_data) {
       printf("✗ WAL enable failed: %s\n", error);
   }

   int main() {
       // Create store and collection...
       const char* wal_config = "{\"path\": \"/tmp/wal\", \"sync_policy\": \"every_write\"}";

       uint64_t task_id = sentinel_collection_enable_wal_async(
           collection, wal_config, on_success, on_error, NULL
       );

       // Wait for completion...
       return 0;
   }
   EOF

   # Build and run test
   gcc -o test_wal test_wal.c -I../include -L. -lsentinel-cxx
   ./test_wal
   ```

## 🛠️ Tools and Scripts

### `bindings/sync_api.py`

**Purpose**: Monitors API changes and handles automatic parts

**Commands**:

```bash
# Create baseline of current API
python3 sync_api.py baseline

# Check for changes since baseline
python3 sync_api.py check

# Auto-update headers for non-breaking changes
python3 sync_api.py update
```

**Capabilities**:

- ✅ Detect API additions/removals
- ✅ Auto-regenerate headers via cbindgen
- ✅ Generate change reports
- ❌ Generate C wrapper implementations

### `cbindgen`

**Purpose**: Auto-generates C headers from Rust `extern "C"` functions

**Configuration**: `crates/sentinel-cxx/cbindgen.toml`

**Triggers**:

- Automatically runs during `cargo build` in `sentinel-cxx` crate
- Output: `crates/sentinel-cxx/target/release/sentinel-cxx.h`

### CMake Build System

**Purpose**: Orchestrates C/C++ compilation and linking

**Key files**:

- `bindings/cxx/CMakeLists.txt` - Main build configuration
- Auto-detects Rust libraries and headers
- Links examples and tests

## 📝 Examples

### Example 1: Adding a Synchronous Method

**Rust API**:

```rust
// sentinel crate
impl Store {
    pub fn get_stats(&self) -> Result<StoreStats> { /* ... */ }
}
```

**C Wrapper**:

```rust
// sentinel-cxx crate
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_store_get_stats(
    store: *mut sentinel_store_t,
    result_json: *mut *mut c_char,
) -> sentinel_error_t {
    if store.is_null() || result_json.is_null() {
        set_error("Store and result pointer cannot be null");
        return sentinel_error_t::SENTINEL_ERROR_NULL_POINTER;
    }

    let store_ref = unsafe { &*(store as *mut Store) };
    match store_ref.get_stats() {
        Ok(stats) => {
            match serde_json::to_string(&stats) {
                Ok(json) => {
                    match CString::new(json) {
                        Ok(cstr) => {
                            unsafe { *result_json = cstr.into_raw() };
                            sentinel_error_t::SENTINEL_OK
                        },
                        Err(_) => {
                            set_error("Failed to convert stats to C string");
                            sentinel_error_t::SENTINEL_ERROR_RUNTIME_ERROR
                        }
                    }
                },
                Err(_) => {
                    set_error("Failed to serialize stats to JSON");
                    sentinel_error_t::SENTINEL_ERROR_JSON_PARSE_ERROR
                }
            }
        },
        Err(e) => {
            set_error(format!("Failed to get stats: {}", e));
            sentinel_error_t::SENTINEL_ERROR_RUNTIME_ERROR
        }
    }
}
```

**Generated C Header** (automatic):

```c
enum sentinel_error_t sentinel_store_get_stats(
    struct sentinel_store_t* store,
    char** result_json
);
```

### Example 2: Adding an Async Method

**Rust API**:

```rust
impl Collection {
    pub async fn compact(&self, options: CompactOptions) -> Result<CompactResult> { /* ... */ }
}
```

**C Wrapper** (with callbacks):

```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sentinel_collection_compact_async(
    collection: *mut sentinel_collection_t,
    options_json: *const c_char,
    callback: DocumentCallback,  // void (*callback)(uint64_t, char*, void*)
    error_callback: ErrorCallback,
    user_data: *mut c_char,
) -> u64 {
    // Implementation similar to WAL example above
    // - Parse JSON options
    // - Spawn async task
    // - Call callbacks on completion
}
```

## 🔧 Troubleshooting

### Common Issues

**Issue**: "undefined reference to `sentinel_new_function`" **Solution**: Check that the C wrapper function is properly
exported with `#[unsafe(no_mangle)]`

**Issue**: Headers not updating after API changes **Solution**:

```bash
# Force rebuild
rm -rf crates/sentinel-cxx/target/
python3 bindings/sync_api.py update
```

**Issue**: Type conversion errors **Solution**: Ensure all Rust types have proper C representations:

- `String` → `char*` (owned)
- `&str` → `const char*` (borrowed)
- `Result<T, E>` → `sentinel_error_t` + output parameters
- Async functions → callback-based APIs

**Issue**: Memory leaks in C wrappers **Solution**: Always follow RAII patterns:

- Use `CString::new().into_raw()` for owned strings
- Call `sentinel_string_free()` in C code
- Document ownership transfer clearly

### Debug Commands

```bash
# Check what symbols are exported
nm target/release/libsentinel_cxx.a | grep sentinel

# Verify header contents
grep -A 5 -B 5 "sentinel_new_function" bindings/cxx/include/sentinel/sentinel-cxx.h

# Test API detection
python3 bindings/sync_api.py check | cat
```

### Getting Help

1. Check existing wrapper implementations in `crates/sentinel-cxx/src/lib.rs`
2. Review the API change report in `bindings/api_changes.md`
3. Test with simple synchronous functions first
4. Use the existing examples as templates

## 📚 Additional Resources

- [Rust FFI Guide](https://doc.rust-lang.org/nomicon/ffi.html)
- [cbindgen Documentation](https://github.com/mozilla/cbindgen)
- [CMake Rust Integration](https://github.com/Devolutions/CMakeRust)
- Existing implementations in `crates/sentinel-cxx/src/lib.rs`

---

**Remember**: The binding update process is semi-automated. API monitoring and header generation are automatic, but C
wrapper implementation requires manual coding expertise. This design ensures type safety and performance while
maintaining a clean C API. Happy coding! 🚀
