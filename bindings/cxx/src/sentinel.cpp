#include "../include/sentinel/sentinel.hpp"
#include <cstring>
#include <string>
#include <vector>
#include <memory>
#include <sstream>
#include <algorithm>

namespace sentinel {

// Store implementation

Store::Store(const std::string& path, const std::string& passphrase) {
    const char* pass_ptr = passphrase.empty() ? nullptr : passphrase.c_str();
    store_ = sentinel_store_new(path.c_str(), pass_ptr);
    if (!store_) {
        throw SentinelException("Failed to create store: " + get_last_error());
    }
}

Store::~Store() {
    if (store_) {
        sentinel_store_free(store_);
        store_ = nullptr;
    }
}

Store::Store(Store&& other) noexcept : store_(other.store_) {
    other.store_ = nullptr;
}

Store& Store::operator=(Store&& other) noexcept {
    if (this != &other) {
        if (store_) {
            sentinel_store_free(store_);
        }
        store_ = other.store_;
        other.store_ = nullptr;
    }
    return *this;
}

std::unique_ptr<Collection> Store::collection(const std::string& name) {
    auto* coll = sentinel_store_collection(store_, name.c_str());
    if (!coll) {
        throw SentinelException("Failed to get collection '" + name + "': " + get_last_error());
    }
    return std::unique_ptr<Collection>(new Collection(coll));
}

void Store::delete_collection(const std::string& name) {
    auto result = sentinel_store_delete_collection(store_, name.c_str());
    if (result != SENTINEL_OK) {
        throw SentinelException("Failed to delete collection '" + name + "': " + get_last_error());
    }
}

std::vector<std::string> Store::list_collections() {
    auto* json_str = sentinel_store_list_collections(store_);
    if (!json_str) {
        throw SentinelException("Failed to list collections: " + get_last_error());
    }

    try {
        auto result = parse_json_array(json_str);
        sentinel_string_free(json_str);
        return result;
    } catch (...) {
        sentinel_string_free(json_str);
        throw;
    }
}

// Collection implementation

Collection::Collection(sentinel_collection_t* collection) : collection_(collection) {
    if (!collection_) {
        throw SentinelException("Collection pointer cannot be null");
    }
}

Collection::~Collection() {
    if (collection_) {
        sentinel_collection_free(collection_);
        collection_ = nullptr;
    }
}

Collection::Collection(Collection&& other) noexcept : collection_(other.collection_) {
    other.collection_ = nullptr;
}

Collection& Collection::operator=(Collection&& other) noexcept {
    if (this != &other) {
        if (collection_) {
            sentinel_collection_free(collection_);
        }
        collection_ = other.collection_;
        other.collection_ = nullptr;
    }
    return *this;
}

void Collection::insert(const std::string& id, const std::string& json_data) {
    auto result = sentinel_collection_insert(collection_, id.c_str(), json_data.c_str());
    if (result != SENTINEL_OK) {
        throw SentinelException("Failed to insert document '" + id + "': " + get_last_error());
    }
}

std::string Collection::get(const std::string& id) {
    auto* json_str = sentinel_collection_get(collection_, id.c_str());
    if (!json_str) {
        throw SentinelException("Failed to get document '" + id + "': " + get_last_error());
    }

    try {
        std::string result(json_str);
        sentinel_string_free(json_str);
        return result;
    } catch (...) {
        sentinel_string_free(json_str);
        throw;
    }
}

void Collection::delete_document(const std::string& id) {
    auto result = sentinel_collection_delete(collection_, id.c_str());
    if (result != SENTINEL_OK) {
        throw SentinelException("Failed to delete document '" + id + "': " + get_last_error());
    }
}

size_t Collection::count() {
    unsigned int count = 0;
    auto result = sentinel_collection_count(collection_, &count);
    if (result != SENTINEL_OK) {
        throw SentinelException("Failed to count documents: " + get_last_error());
    }
    return static_cast<size_t>(count);
}

void Collection::update(const std::string& id, const std::string& json_data) {
    auto result = sentinel_collection_update(collection_, id.c_str(), json_data.c_str());
    if (result != SENTINEL_OK) {
        throw SentinelException("Failed to update document '" + id + "': " + get_last_error());
    }
}

bool Collection::upsert(const std::string& id, const std::string& json_data) {
    bool was_insert = false;
    auto result = sentinel_collection_upsert(collection_, id.c_str(), json_data.c_str(), &was_insert);
    if (result != SENTINEL_OK) {
        throw SentinelException("Failed to upsert document '" + id + "': " + get_last_error());
    }
    return was_insert;
}

// Utility functions

std::vector<std::string> parse_json_array(const std::string& json_str) {
    std::vector<std::string> result;

    // Simple JSON array parser for string arrays like ["item1", "item2"]
    if (json_str.empty() || json_str[0] != '[' || json_str.back() != ']') {
        throw SentinelException("Invalid JSON array format: " + json_str);
    }

    std::string content = json_str.substr(1, json_str.size() - 2);
    if (content.empty()) {
        return result; // Empty array
    }

    std::stringstream ss(content);
    std::string item;
    bool in_string = false;
    char prev_char = '\0';

    for (char c : content) {
        if (c == '"' && prev_char != '\\') {
            in_string = !in_string;
            item += c;
        } else if (c == ',' && !in_string) {
            if (!item.empty()) {
                // Remove surrounding quotes if present
                if (item.size() >= 2 && item.front() == '"' && item.back() == '"') {
                    item = item.substr(1, item.size() - 2);
                }
                result.push_back(item);
                item.clear();
            }
        } else {
            item += c;
        }
        prev_char = c;
    }

    // Add the last item
    if (!item.empty()) {
        if (item.size() >= 2 && item.front() == '"' && item.back() == '"') {
            item = item.substr(1, item.size() - 2);
        }
        result.push_back(item);
    }

    return result;
}

std::string get_last_error() {
    auto* error_ptr = sentinel_get_last_error();
    if (!error_ptr) {
        return "Unknown error";
    }

    std::string error(error_ptr);
    sentinel_string_free(error_ptr);
    return error;
}

} // namespace sentinel