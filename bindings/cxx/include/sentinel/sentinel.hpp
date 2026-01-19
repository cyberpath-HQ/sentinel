#pragma once

#include <string>
#include <vector>
#include <memory>
#include <stdexcept>

#ifdef _WIN32
    #ifdef SENTINEL_CXX_EXPORTS
        #define SENTINEL_CXX_API __declspec(dllexport)
    #else
        #define SENTINEL_CXX_API __declspec(dllimport)
    #endif
#else
    #define SENTINEL_CXX_API __attribute__((visibility("default")))
#endif

// Forward declarations of C types
extern "C" {
    struct sentinel_store_t;
    struct sentinel_collection_t;
    typedef enum sentinel_error_t {
        SENTINEL_OK = 0,
        SENTINEL_ERROR_NULL_POINTER = 1,
        SENTINEL_ERROR_INVALID_ARGUMENT = 2,
        SENTINEL_ERROR_IO_ERROR = 3,
        SENTINEL_ERROR_RUNTIME_ERROR = 4,
        SENTINEL_ERROR_JSON_PARSE_ERROR = 5,
        SENTINEL_ERROR_NOT_FOUND = 6,
    } sentinel_error_t;

    // Store functions
    sentinel_store_t* sentinel_store_new(const char* path, const char* passphrase);
    void sentinel_store_free(sentinel_store_t* store);
    sentinel_collection_t* sentinel_store_collection(sentinel_store_t* store, const char* name);
    sentinel_error_t sentinel_store_delete_collection(sentinel_store_t* store, const char* name);
    char* sentinel_store_list_collections(sentinel_store_t* store);
    char* sentinel_get_last_error();
    void sentinel_string_free(char* str);

    // Collection functions
    void sentinel_collection_free(sentinel_collection_t* collection);
    sentinel_error_t sentinel_collection_insert(sentinel_collection_t* collection, const char* id, const char* json_data);
    char* sentinel_collection_get(sentinel_collection_t* collection, const char* id);
    sentinel_error_t sentinel_collection_delete(sentinel_collection_t* collection, const char* id);
    sentinel_error_t sentinel_collection_count(sentinel_collection_t* collection, unsigned int* count);
    sentinel_error_t sentinel_collection_update(sentinel_collection_t* collection, const char* id, const char* json_data);
    sentinel_error_t sentinel_collection_upsert(sentinel_collection_t* collection, const char* id, const char* json_data, bool* was_insert);
}

namespace sentinel {

/// Exception thrown by Sentinel operations
class SENTINEL_CXX_API SentinelException : public std::runtime_error {
public:
    explicit SentinelException(const std::string& message)
        : std::runtime_error(message) {}
};

/// Forward declaration
class Collection;

/// RAII wrapper for Sentinel Store
class SENTINEL_CXX_API Store {
public:
    /// Create a new store at the specified path
    explicit Store(const std::string& path, const std::string& passphrase = "");

    /// Destructor
    ~Store();

    // Disable copy
    Store(const Store&) = delete;
    Store& operator=(const Store&) = delete;

    // Enable move
    Store(Store&& other) noexcept;
    Store& operator=(Store&& other) noexcept;

    /// Get a collection from the store
    std::unique_ptr<Collection> collection(const std::string& name);

    /// Delete a collection from the store
    void delete_collection(const std::string& name);

    /// List all collections in the store
    std::vector<std::string> list_collections();

private:
    sentinel_store_t* store_;
};

/// RAII wrapper for Sentinel Collection
class SENTINEL_CXX_API Collection {
public:
    /// Constructor - takes ownership of the C collection pointer
    explicit Collection(sentinel_collection_t* collection);

    /// Destructor
    ~Collection();

    // Disable copy
    Collection(const Collection&) = delete;
    Collection& operator=(const Collection&) = delete;

    // Enable move
    Collection(Collection&& other) noexcept;
    Collection& operator=(Collection&& other) noexcept;

    /// Insert a document into the collection
    void insert(const std::string& id, const std::string& json_data);

    /// Get a document by ID
    std::string get(const std::string& id);

    /// Delete a document by ID
    void delete_document(const std::string& id);

    /// Get the count of documents in the collection
    size_t count();

    /// Update a document
    void update(const std::string& id, const std::string& json_data);

    /// Upsert a document (insert or update)
    bool upsert(const std::string& id, const std::string& json_data);

private:
    sentinel_collection_t* collection_;
};

/// Utility functions

/// Parse JSON array string to vector of strings
SENTINEL_CXX_API std::vector<std::string> parse_json_array(const std::string& json_str);

/// Get the last error message
SENTINEL_CXX_API std::string get_last_error();

} // namespace sentinel