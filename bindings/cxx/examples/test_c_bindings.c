#include "sentinel-cxx.h"
#include <stdio.h>
#include <stdlib.h>

int main() {
    printf("Testing Cyberpath Sentinel C bindings...\n");

    // Test store creation
    printf("Creating store...\n");
    sentinel_store_t* store = sentinel_store_new("./test_store", NULL);
    if (!store) {
        const char* error = sentinel_get_last_error();
        fprintf(stderr, "Failed to create store: %s\n", error ? error : "Unknown error");
        sentinel_string_free(error);
        return 1;
    }
    printf("✓ Store created successfully\n");

    // Test collection creation
    printf("Creating collection...\n");
    sentinel_collection_t* collection = sentinel_store_collection(store, "test_collection");
    if (!collection) {
        const char* error = sentinel_get_last_error();
        fprintf(stderr, "Failed to create collection: %s\n", error ? error : "Unknown error");
        sentinel_string_free(error);
        sentinel_store_free(store);
        return 1;
    }
    printf("✓ Collection created successfully\n");

    // Test document insertion
    printf("Inserting document...\n");
    const char* json_data = "{\"name\": \"Test Document\", \"value\": 42}";
    sentinel_error_t result = sentinel_collection_insert(collection, "doc1", json_data);
    if (result != SENTINEL_OK) {
        const char* error = sentinel_get_last_error();
        fprintf(stderr, "Failed to insert document: %s\n", error ? error : "Unknown error");
        sentinel_string_free(error);
        sentinel_collection_free(collection);
        sentinel_store_free(store);
        return 1;
    }
    printf("✓ Document inserted successfully\n");

    // Test document retrieval
    printf("Retrieving document...\n");
    char* retrieved_data = sentinel_collection_get(collection, "doc1");
    if (!retrieved_data) {
        const char* error = sentinel_get_last_error();
        fprintf(stderr, "Failed to retrieve document: %s\n", error ? error : "Unknown error");
        sentinel_string_free(error);
        sentinel_collection_free(collection);
        sentinel_store_free(store);
        return 1;
    }
    printf("✓ Document retrieved: %s\n", retrieved_data);
    sentinel_string_free(retrieved_data);

    // Test collection count
    printf("Getting collection count...\n");
    unsigned int count = 0;
    result = sentinel_collection_count(collection, &count);
    if (result != SENTINEL_OK) {
        const char* error = sentinel_get_last_error();
        fprintf(stderr, "Failed to get count: %s\n", error ? error : "Unknown error");
        sentinel_string_free(error);
        sentinel_collection_free(collection);
        sentinel_store_free(store);
        return 1;
    }
    printf("✓ Collection has %u documents\n", count);

    // Cleanup
    sentinel_collection_free(collection);
    sentinel_store_free(store);

    printf("✓ All tests passed!\n");
    return 0;
}