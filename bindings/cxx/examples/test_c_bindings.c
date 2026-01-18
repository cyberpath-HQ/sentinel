#include <sentinel-cxx.h>
#include <stdio.h>
#include <stdlib.h>

int main() {
    printf("Testing Cyberpath Sentinel C bindings...\n");

    // Test store creation
    printf("1. Creating store...\n");
    sentinel_store_t* store = sentinel_store_new("./test_db", NULL);
    if (!store) {
        fprintf(stderr, "FAILED: Could not create store\n");
        const char* error = sentinel_get_last_error();
        if (error) {
            fprintf(stderr, "Error: %s\n", error);
            sentinel_string_free(error);
        }
        return 1;
    }
    printf("✓ Store created successfully\n");

    // Test collection creation
    printf("2. Creating collection...\n");
    sentinel_collection_t* collection = sentinel_store_collection(store, "test_collection");
    if (!collection) {
        fprintf(stderr, "FAILED: Could not create collection\n");
        const char* error = sentinel_get_last_error();
        if (error) {
            fprintf(stderr, "Error: %s\n", error);
            sentinel_string_free(error);
        }
        sentinel_store_free(store);
        return 1;
    }
    printf("✓ Collection created successfully\n");

    // Test document insertion
    printf("3. Inserting document...\n");
    sentinel_error_t result = sentinel_collection_insert(collection, "doc1",
        "{\"message\": \"Hello from C bindings!\", \"timestamp\": 1234567890}");
    if (result != SENTINEL_OK) {
        fprintf(stderr, "FAILED: Could not insert document\n");
        const char* error = sentinel_get_last_error();
        if (error) {
            fprintf(stderr, "Error: %s\n", error);
            sentinel_string_free(error);
        }
        sentinel_collection_free(collection);
        sentinel_store_free(store);
        return 1;
    }
    printf("✓ Document inserted successfully\n");

    // Test document retrieval
    printf("4. Retrieving document...\n");
    char* doc_json = sentinel_collection_get(collection, "doc1");
    if (!doc_json) {
        fprintf(stderr, "FAILED: Could not retrieve document\n");
        const char* error = sentinel_get_last_error();
        if (error) {
            fprintf(stderr, "Error: %s\n", error);
            sentinel_string_free(error);
        }
        sentinel_collection_free(collection);
        sentinel_store_free(store);
        return 1;
    }
    printf("✓ Document retrieved: %s\n", doc_json);
    sentinel_string_free(doc_json);

    // Cleanup
    printf("5. Cleaning up...\n");
    sentinel_collection_free(collection);
    sentinel_store_free(store);
    printf("✓ Cleanup completed\n");

    printf("\nAll tests passed! C bindings are working correctly.\n");
    return 0;
}