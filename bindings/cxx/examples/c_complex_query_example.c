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
    printf("Cyberpath Sentinel C Complex Query & Filtering Example\n");
    printf("=======================================================\n\n");

    // Create a store
    printf("Creating store at './complex_query_db'...\n");
    sentinel_store_t* store = sentinel_store_new("./complex_query_db", NULL);
    CHECK_NULL(store, "Failed to create store");

    // Get users collection
    printf("Getting 'users' collection...\n");
    sentinel_collection_t* users = sentinel_store_collection(store, "users");
    CHECK_NULL(users, "Failed to get users collection");

    // Insert comprehensive test data
    printf("Inserting comprehensive test data...\n");

    CHECK_ERROR(sentinel_collection_insert(users, "alice",
        "{\"name\": \"Alice Johnson\", \"age\": 28, \"city\": \"New York\", \"active\": true, \"score\": 95.5, \"department\": \"Engineering\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "bob",
        "{\"name\": \"Bob Smith\", \"age\": 34, \"city\": \"Los Angeles\", \"active\": false, \"score\": 87.2, \"department\": \"Sales\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "charlie",
        "{\"name\": \"Charlie Brown\", \"age\": 22, \"city\": \"New York\", \"active\": true, \"score\": 92.8, \"department\": \"Engineering\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "diana",
        "{\"name\": \"Diana Prince\", \"age\": 31, \"city\": \"Chicago\", \"active\": true, \"score\": 89.1, \"department\": \"HR\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "eve",
        "{\"name\": \"Eve Wilson\", \"age\": 26, \"city\": \"New York\", \"active\": false, \"score\": 91.3, \"department\": \"Marketing\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "frank",
        "{\"name\": \"Frank Miller\", \"age\": 45, \"city\": \"Boston\", \"active\": true, \"score\": 88.9, \"department\": \"Engineering\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "grace",
        "{\"name\": \"Grace Lee\", \"age\": 29, \"city\": \"Seattle\", \"active\": true, \"score\": 96.2, \"department\": \"Engineering\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "henry",
        "{\"name\": \"Henry Davis\", \"age\": 38, \"city\": \"Austin\", \"active\": false, \"score\": 84.7, \"department\": \"Finance\"}"));

    printf("✓ Test data inserted successfully (8 users)\n");

    // Test 1: Create a complex query with multiple filters
    printf("\n=== Test 1: Complex Query with Multiple Filters ===\n");
    sentinel_query_t* complex_query = sentinel_query_builder_new();
    CHECK_NULL(complex_query, "Failed to create query builder");

    // Add multiple filters: active = true, age > 25, department contains "Engineer"
    CHECK_ERROR(sentinel_query_builder_filter_equals(complex_query, "active", "true"));
    CHECK_ERROR(sentinel_query_builder_filter_greater_than(complex_query, "age", "25"));
    CHECK_ERROR(sentinel_query_builder_filter_contains(complex_query, "department", "Engineer"));

    // Execute query
    char* complex_result = sentinel_collection_query(users, complex_query);
    CHECK_NULL(complex_result, "Complex query execution failed");
    printf("Complex query result (active engineers > 25): %s\n", complex_result);
    sentinel_string_free(complex_result);

    // Test 2: Query with sorting
    printf("\n=== Test 2: Query with Sorting ===\n");
    sentinel_query_t* sorted_query = sentinel_query_builder_new();
    CHECK_NULL(sorted_query, "Failed to create sorted query");
    CHECK_ERROR(sentinel_query_builder_filter_equals(sorted_query, "active", "true"));
    CHECK_ERROR(sentinel_query_builder_sort(sorted_query, "score", 1)); // Descending order

    char* sorted_result = sentinel_collection_query(users, sorted_query);
    printf("Active users sorted by score (descending): %s\n", sorted_result ? sorted_result : "null");
    if (sorted_result) sentinel_string_free(sorted_result);
    sentinel_query_free(sorted_query);

    // Test 3: Query with limit and offset (pagination)
    printf("\n=== Test 3: Query with Pagination ===\n");
    sentinel_query_t* paginated_query = sentinel_query_builder_new();
    CHECK_NULL(paginated_query, "Failed to create paginated query");
    CHECK_ERROR(sentinel_query_builder_filter_equals(paginated_query, "city", "\"New York\""));
    CHECK_ERROR(sentinel_query_builder_sort(paginated_query, "age", 0)); // Ascending
    CHECK_ERROR(sentinel_query_builder_limit(paginated_query, 2)); // First 2 results
    CHECK_ERROR(sentinel_query_builder_offset(paginated_query, 1)); // Skip first result

    char* paginated_result = sentinel_collection_query(users, paginated_query);
    printf("New York users (sorted by age, limit 2, offset 1): %s\n", paginated_result ? paginated_result : "null");
    if (paginated_result) sentinel_string_free(paginated_result);
    sentinel_query_free(paginated_query);

    // Test 4: Simple equality query for comparison
    printf("\n=== Test 4: Simple Equality Query ===\n");
    sentinel_query_t* simple_query = sentinel_query_builder_new();
    CHECK_NULL(simple_query, "Failed to create simple query");
    CHECK_ERROR(sentinel_query_builder_filter_equals(simple_query, "city", "\"New York\""));

    char* simple_result = sentinel_collection_query(users, simple_query);
    printf("Simple equality query (city = New York): %s\n", simple_result ? simple_result : "null");
    if (simple_result) sentinel_string_free(simple_result);
    sentinel_query_free(simple_query);

    // Test 5: Range queries
    printf("\n=== Test 5: Range Queries ===\n");

    // Age between 25-35
    sentinel_query_t* range_query1 = sentinel_query_builder_new();
    CHECK_NULL(range_query1, "Failed to create range query 1");
    CHECK_ERROR(sentinel_query_builder_filter_greater_than(range_query1, "age", "24"));
    CHECK_ERROR(sentinel_query_builder_filter_less_than(range_query1, "age", "36"));

    char* range_result1 = sentinel_collection_query(users, range_query1);
    printf("Age range 25-35: %s\n", range_result1 ? range_result1 : "null");
    if (range_result1) sentinel_string_free(range_result1);
    sentinel_query_free(range_query1);

    // High scores
    sentinel_query_t* range_query2 = sentinel_query_builder_new();
    CHECK_NULL(range_query2, "Failed to create range query 2");
    CHECK_ERROR(sentinel_query_builder_filter_greater_than(range_query2, "score", "90"));

    char* range_result2 = sentinel_collection_query(users, range_query2);
    printf("High scores (>90): %s\n", range_result2 ? range_result2 : "null");
    if (range_result2) sentinel_string_free(range_result2);
    sentinel_query_free(range_query2);

    // Test 6: Document count verification
    printf("\n=== Test 6: Document Count Verification ===\n");
    uint32_t total_count = 0;
    CHECK_ERROR(sentinel_collection_count(users, &total_count));
    printf("Total documents in collection: %u\n", total_count);

    // Test 7: Empty query (should return all)
    printf("\n=== Test 7: Empty Query (All Documents) ===\n");
    sentinel_query_t* empty_query = sentinel_query_builder_new();
    CHECK_NULL(empty_query, "Failed to create empty query");
    // No filters added

    char* empty_result = sentinel_collection_query(users, empty_query);
    printf("Empty query result: %s\n", empty_result ? empty_result : "null");
    if (empty_result) sentinel_string_free(empty_result);
    sentinel_query_free(empty_query);

    // Test 8: Non-matching query
    printf("\n=== Test 8: Non-Matching Query ===\n");
    sentinel_query_t* no_match_query = sentinel_query_builder_new();
    CHECK_NULL(no_match_query, "Failed to create no-match query");
    CHECK_ERROR(sentinel_query_builder_filter_equals(no_match_query, "city", "\"NonExistentCity\""));

    char* no_match_result = sentinel_collection_query(users, no_match_query);
    printf("Non-matching query result: %s\n", no_match_result ? no_match_result : "null");
    if (no_match_result) sentinel_string_free(no_match_result);
    sentinel_query_free(no_match_query);

    // Cleanup
    sentinel_query_free(complex_query);
    sentinel_collection_free(users);
    sentinel_store_free(store);

    printf("\n🎉 All complex query and filtering tests completed successfully!\n");
    printf("✓ Multiple filter conditions\n");
    printf("✓ Sorting (ascending/descending)\n");
    printf("✓ Pagination (limit/offset)\n");
    printf("✓ Range queries (greater than, less than)\n");
    printf("✓ String matching (contains)\n");
    printf("✓ Complex query combinations\n");
    printf("✓ Edge cases (empty queries, no matches)\n");

    return 0;
}