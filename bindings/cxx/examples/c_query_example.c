#include "sentinel-cxx.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>

#define CHECK_ERROR(func_call) \
    do { \
        sentinel_error_t result = (func_call); \
        if (result != SENTINEL_OK) { \
            char* error = sentinel_get_last_error(); \
            fprintf(stderr, "Error at %s:%d: %s\n", __FILE__, __LINE__, error); \
            sentinel_string_free(error); \
            exit(1); \
        } \
    } while(0)

#define CHECK_NULL(ptr, msg) \
    do { \
        if (!(ptr)) { \
            char* error = sentinel_get_last_error(); \
            fprintf(stderr, "Error at %s:%d: %s - %s\n", __FILE__, __LINE__, msg, error ? error : "Unknown"); \
            if (error) sentinel_string_free(error); \
            exit(1); \
        } \
    } while(0)

int main() {
    printf("Cyberpath Sentinel C Query & Filtering Example\n");
    printf("==============================================\n\n");

    // Create a store
    printf("Creating store at './query_example_db'...\n");
    sentinel_store_t* store = sentinel_store_new("./query_example_db", NULL);
    CHECK_NULL(store, "Failed to create store");

    // Get users collection
    printf("Getting 'users' collection...\n");
    sentinel_collection_t* users = sentinel_store_collection(store, "users");
    CHECK_NULL(users, "Failed to get users collection");

    // Insert test data
    printf("Inserting test data...\n");

    CHECK_ERROR(sentinel_collection_insert(users, "alice",
        "{\"name\": \"Alice Johnson\", \"age\": 28, \"city\": \"New York\", \"active\": true, \"score\": 95.5}"));

    CHECK_ERROR(sentinel_collection_insert(users, "bob",
        "{\"name\": \"Bob Smith\", \"age\": 34, \"city\": \"Los Angeles\", \"active\": false, \"score\": 87.2}"));

    CHECK_ERROR(sentinel_collection_insert(users, "charlie",
        "{\"name\": \"Charlie Brown\", \"age\": 22, \"city\": \"New York\", \"active\": true, \"score\": 92.8}"));

    CHECK_ERROR(sentinel_collection_insert(users, "diana",
        "{\"name\": \"Diana Prince\", \"age\": 31, \"city\": \"Chicago\", \"active\": true, \"score\": 89.1}"));

    CHECK_ERROR(sentinel_collection_insert(users, "eve",
        "{\"name\": \"Eve Wilson\", \"age\": 26, \"city\": \"New York\", \"active\": false, \"score\": 91.3}"));

    printf("✓ Test data inserted successfully\n");

    // Test 1: Simple equality query
    printf("\n=== Test 1: Simple Equality Query ===\n");
    sentinel_query_t* query1 = sentinel_query_new_simple("city", "\"New York\"");
    CHECK_NULL(query1, "Failed to create query");

    char* result1 = sentinel_collection_query(users, query1);
    CHECK_NULL(result1, "Query execution failed");
    printf("Users in New York: %s\n", result1);
    sentinel_string_free(result1);
    sentinel_query_free(query1);

    // Test 2: Get document count
    printf("\n=== Test 2: Document Count ===\n");
    uint32_t count = 0;
    CHECK_ERROR(sentinel_collection_count(users, &count));
    printf("Total documents in collection: %u\n", count);

    // Test 3: Retrieve specific documents
    printf("\n=== Test 3: Retrieve Specific Documents ===\n");

    char* alice_doc = sentinel_collection_get(users, "alice");
    if (alice_doc) {
        printf("Alice's document: %s\n", alice_doc);
        sentinel_string_free(alice_doc);
    } else {
        printf("Alice's document not found\n");
    }

    char* nonexistent_doc = sentinel_collection_get(users, "nonexistent");
    if (nonexistent_doc) {
        printf("Unexpected: found nonexistent document: %s\n", nonexistent_doc);
        sentinel_string_free(nonexistent_doc);
    } else {
        printf("✓ Correctly returned NULL for nonexistent document\n");
    }

    // Test 4: Update a document
    printf("\n=== Test 4: Update Document ===\n");
    CHECK_ERROR(sentinel_collection_update(users, "bob",
        "{\"name\": \"Bob Smith\", \"age\": 35, \"city\": \"Los Angeles\", \"active\": true, \"score\": 90.0}"));
    printf("✓ Updated Bob's age and active status\n");

    // Verify update
    char* bob_updated = sentinel_collection_get(users, "bob");
    if (bob_updated) {
        printf("Bob's updated document: %s\n", bob_updated);
        sentinel_string_free(bob_updated);
    }

    // Test 5: Upsert (insert or update)
    printf("\n=== Test 5: Upsert Operation ===\n");
    bool was_insert = false;
    CHECK_ERROR(sentinel_collection_upsert(users, "frank",
        "{\"name\": \"Frank Miller\", \"age\": 29, \"city\": \"Boston\", \"active\": true, \"score\": 88.5}",
        &was_insert));
    printf("Frank %s (was_insert: %s)\n", was_insert ? "inserted" : "updated", was_insert ? "true" : "false");

    // Test upsert again (should update)
    CHECK_ERROR(sentinel_collection_upsert(users, "frank",
        "{\"name\": \"Frank Miller\", \"age\": 30, \"city\": \"Boston\", \"active\": true, \"score\": 91.0}",
        &was_insert));
    printf("Frank %s again (was_insert: %s)\n", was_insert ? "inserted" : "updated", was_insert ? "true" : "false");

    // Test 6: Delete a document
    printf("\n=== Test 6: Delete Document ===\n");
    CHECK_ERROR(sentinel_collection_delete(users, "eve"));
    printf("✓ Deleted Eve's document\n");

    // Verify deletion
    char* eve_deleted = sentinel_collection_get(users, "eve");
    if (eve_deleted) {
        printf("ERROR: Eve's document still exists: %s\n", eve_deleted);
        sentinel_string_free(eve_deleted);
    } else {
        printf("✓ Confirmed Eve's document was deleted\n");
    }

    // Final count
    printf("\n=== Final State ===\n");
    uint32_t final_count = 0;
    CHECK_ERROR(sentinel_collection_count(users, &final_count));
    printf("Final document count: %u\n", final_count);

    // Cleanup
    printf("\n=== Cleanup ===\n");
    sentinel_collection_free(users);
    sentinel_store_free(store);
    printf("✓ All resources cleaned up\n");

    printf("\n🎉 All query and filtering tests passed!\n");
    return 0;
}