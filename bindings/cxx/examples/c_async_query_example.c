#include "sentinel-cxx.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>
#include <unistd.h> // for sleep

// Global variables for async callbacks
static int async_tests_completed = 0;
static const int TOTAL_ASYNC_TESTS = 6;

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

// Async callback functions
void on_store_created(uint64_t task_id, sentinel_store_t* store, char* user_data) {
    printf("✓ Async store creation completed (task %llu)\n", task_id);
    if (user_data) free(user_data);
    async_tests_completed++;
}

void on_collection_created(uint64_t task_id, sentinel_collection_t* collection, char* user_data) {
    printf("✓ Async collection creation completed (task %llu)\n", task_id);
    if (user_data) free(user_data);
    async_tests_completed++;
}

void on_document_inserted(uint64_t task_id, char* user_data) {
    printf("✓ Async document insertion completed (task %llu)\n", task_id);
    if (user_data) free(user_data);
    async_tests_completed++;
}

void on_document_updated(uint64_t task_id, char* user_data) {
    printf("✓ Async document update completed (task %llu)\n", task_id);
    if (user_data) free(user_data);
    async_tests_completed++;
}

void on_document_upserted(uint64_t task_id, bool was_insert, char* user_data) {
    printf("✓ Async document upsert completed (task %llu, was_insert: %s)\n",
           task_id, was_insert ? "true" : "false");
    if (user_data) free(user_data);
    async_tests_completed++;
}

void on_document_deleted(uint64_t task_id, char* user_data) {
    printf("✓ Async document deletion completed (task %llu)\n", task_id);
    if (user_data) free(user_data);
    async_tests_completed++;
}

void on_query_completed(uint64_t task_id, char* result, char* user_data) {
    printf("✓ Async query completed (task %llu): %s\n", task_id, result ? result : "NULL");
    if (result) sentinel_string_free(result);
    if (user_data) free(user_data);
    async_tests_completed++;
}

void on_count_completed(uint64_t task_id, uint32_t count, char* user_data) {
    printf("✓ Async count completed (task %llu): %u documents\n", task_id, count);
    if (user_data) free(user_data);
    async_tests_completed++;
}

void on_error(uint64_t task_id, const char* error, char* user_data) {
    fprintf(stderr, "✗ Async operation failed (task %llu): %s\n", task_id, error);
    if (user_data) free(user_data);
    async_tests_completed++;
}

int main() {
    printf("Cyberpath Sentinel C Async Query & Filtering Example\n");
    printf("=====================================================\n\n");

    // Reset test counter
    async_tests_completed = 0;

    printf("Starting async operations...\n\n");

    // Test 1: Create store asynchronously
    printf("=== Test 1: Async Store Creation ===\n");
    uint64_t store_task = sentinel_store_new_async(
        "./async_query_example_db",
        NULL,
        on_store_created,
        on_error,
        strdup("store_test")
    );
    printf("Store creation task ID: %llu\n", store_task);

    // Wait a bit for store to be created, then create collection
    sleep(1);

    // For demo purposes, create a store synchronously to get collection
    // In real async code, you'd chain these operations
    sentinel_store_t* store = sentinel_store_new("./async_query_example_db", NULL);
    if (!store) {
        fprintf(stderr, "Failed to create store for demo\n");
        return 1;
    }

    sentinel_collection_t* users = sentinel_store_collection(store, "users");
    if (!users) {
        fprintf(stderr, "Failed to get users collection\n");
        sentinel_store_free(store);
        return 1;
    }

    // Insert test data synchronously for demo
    CHECK_ERROR(sentinel_collection_insert(users, "async_alice",
        "{\"name\": \"Async Alice\", \"age\": 25, \"city\": \"Seattle\", \"active\": true}"));
    CHECK_ERROR(sentinel_collection_insert(users, "async_bob",
        "{\"name\": \"Async Bob\", \"age\": 30, \"city\": \"Portland\", \"active\": false}"));

    // Test 2: Insert document asynchronously
    printf("\n=== Test 2: Async Document Insertion ===\n");
    uint64_t insert_task = sentinel_collection_insert_async(
        users,
        "async_charlie",
        "{\"name\": \"Async Charlie\", \"age\": 27, \"city\": \"Seattle\", \"active\": true}",
        on_document_inserted,
        on_error,
        strdup("insert_test")
    );
    printf("Insert task ID: %llu\n", insert_task);

    // Test 3: Update document asynchronously
    printf("\n=== Test 3: Async Document Update ===\n");
    uint64_t update_task = sentinel_collection_update_async(
        users,
        "async_bob",
        "{\"name\": \"Async Bob\", \"age\": 31, \"city\": \"Portland\", \"active\": true}",
        on_document_updated,
        on_error,
        strdup("update_test")
    );
    printf("Update task ID: %llu\n", update_task);

    // Test 4: Upsert document asynchronously
    printf("\n=== Test 4: Async Document Upsert ===\n");
    uint64_t upsert_task = sentinel_collection_upsert_async(
        users,
        "async_diana",
        "{\"name\": \"Async Diana\", \"age\": 28, \"city\": \"Seattle\", \"active\": true}",
        on_document_upserted,
        on_error,
        strdup("upsert_test")
    );
    printf("Upsert task ID: %llu\n", upsert_task);

    // Test 5: Delete document asynchronously
    printf("\n=== Test 5: Async Document Deletion ===\n");
    uint64_t delete_task = sentinel_collection_delete_async(
        users,
        "async_alice",
        on_document_deleted,
        on_error,
        strdup("delete_test")
    );
    printf("Delete task ID: %llu\n", delete_task);

    // Test 6: Query documents asynchronously
    printf("\n=== Test 6: Async Query ===\n");
    sentinel_query_t* query = sentinel_query_new_simple("city", "\"Seattle\"");
    if (query) {
        uint64_t query_task = sentinel_collection_query_async(
            users,
            query,
            on_query_completed,
            on_error,
            strdup("query_test")
        );
        printf("Query task ID: %llu\n", query_task);
        sentinel_query_free(query);
    } else {
        printf("Failed to create query\n");
        async_tests_completed++; // Count as failed
    }

    // Test 7: Count documents asynchronously
    printf("\n=== Test 7: Async Count ===\n");
    uint64_t count_task = sentinel_collection_count_async(
        users,
        on_count_completed,
        on_error,
        strdup("count_test")
    );
    printf("Count task ID: %llu\n", count_task);

    // Wait for all async operations to complete
    printf("\n=== Waiting for Async Operations ===\n");
    int expected_tests = 7; // 1 store + 1 insert + 1 update + 1 upsert + 1 delete + 1 query + 1 count
    while (async_tests_completed < expected_tests) {
        printf("Completed: %d/%d tests\n", async_tests_completed, expected_tests);
        sleep(1);
    }

    printf("\n🎉 All async query and filtering tests completed!\n");
    printf("Total async operations: %d\n", async_tests_completed);

    // Cleanup
    sentinel_collection_free(users);
    sentinel_store_free(store);

    return 0;
}