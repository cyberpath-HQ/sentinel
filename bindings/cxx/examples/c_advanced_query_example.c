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
    printf("Cyberpath Sentinel C Advanced Query & Logical Operations Example\n");
    printf("=================================================================\n\n");

    // Create a store
    printf("Creating store at './advanced_query_db'...\n");
    sentinel_store_t* store = sentinel_store_new("./advanced_query_db", NULL);
    CHECK_NULL(store, "Failed to create store");

    // Get users collection
    printf("Getting 'users' collection...\n");
    sentinel_collection_t* users = sentinel_store_collection(store, "users");
    CHECK_NULL(users, "Failed to get users collection");

    // Insert comprehensive test data with various field types
    printf("Inserting comprehensive test data...\n");

    CHECK_ERROR(sentinel_collection_insert(users, "alice",
        "{\"name\": \"Alice Johnson\", \"age\": 28, \"city\": \"New York\", \"active\": true, \"score\": 95.5, \"department\": \"Engineering\", \"tags\": [\"developer\", \"senior\"], \"level\": \"senior\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "bob",
        "{\"name\": \"Bob Smith\", \"age\": 34, \"city\": \"Los Angeles\", \"active\": false, \"score\": 87.2, \"department\": \"Sales\", \"tags\": [\"sales\", \"manager\"], \"level\": \"manager\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "charlie",
        "{\"name\": \"Charlie Brown\", \"age\": 22, \"city\": \"New York\", \"active\": true, \"score\": 92.8, \"department\": \"Engineering\", \"tags\": [\"developer\", \"junior\"], \"level\": \"junior\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "diana",
        "{\"name\": \"Diana Prince\", \"age\": 31, \"city\": \"Chicago\", \"active\": true, \"score\": 89.1, \"department\": \"HR\", \"tags\": [\"hr\", \"manager\"], \"level\": \"manager\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "eve",
        "{\"name\": \"Eve Wilson\", \"age\": 26, \"city\": \"New York\", \"active\": false, \"score\": 91.3, \"department\": \"Marketing\", \"tags\": [\"marketing\", \"specialist\"], \"level\": \"specialist\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "frank",
        "{\"name\": \"Frank Miller\", \"age\": 45, \"city\": \"Boston\", \"active\": true, \"score\": 88.9, \"department\": \"Engineering\", \"tags\": [\"architect\", \"senior\"], \"level\": \"senior\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "grace",
        "{\"name\": \"Grace Lee\", \"age\": 29, \"city\": \"Seattle\", \"active\": true, \"score\": 96.2, \"department\": \"Engineering\", \"tags\": [\"developer\", \"senior\"], \"level\": \"senior\"}"));

    CHECK_ERROR(sentinel_collection_insert(users, "henry",
        "{\"name\": \"Henry Davis\", \"age\": 38, \"city\": \"Austin\", \"active\": false, \"score\": 84.7, \"department\": \"Finance\", \"tags\": [\"finance\", \"analyst\"], \"level\": \"analyst\"}"));

    printf("✓ Test data inserted successfully (8 users)\n");

    // Test 1: All comparison operators
    printf("\n=== Test 1: Comparison Operators ===\n");

    // Greater than or equal
    sentinel_query_t* gte_query = sentinel_query_builder_new();
    CHECK_NULL(gte_query, "Failed to create query builder");
    CHECK_ERROR(sentinel_query_builder_filter_greater_or_equal(gte_query, "age", "30"));
    char* gte_result = sentinel_collection_query(users, gte_query);
    printf("Age >= 30: %s\n", gte_result ? gte_result : "null");
    if (gte_result) sentinel_string_free(gte_result);
    sentinel_query_free(gte_query);

    // Less than or equal
    sentinel_query_t* lte_query = sentinel_query_builder_new();
    CHECK_NULL(lte_query, "Failed to create query builder");
    CHECK_ERROR(sentinel_query_builder_filter_less_or_equal(lte_query, "score", "90"));
    char* lte_result = sentinel_collection_query(users, lte_query);
    printf("Score <= 90: %s\n", lte_result ? lte_result : "null");
    if (lte_result) sentinel_string_free(lte_result);
    sentinel_query_free(lte_query);

    // Test 2: String matching operators
    printf("\n=== Test 2: String Matching Operators ===\n");

    // Starts with
    sentinel_query_t* starts_query = sentinel_query_builder_new();
    CHECK_NULL(starts_query, "Failed to create query builder");
    CHECK_ERROR(sentinel_query_builder_filter_starts_with(starts_query, "name", "A"));
    char* starts_result = sentinel_collection_query(users, starts_query);
    printf("Names starting with 'A': %s\n", starts_result ? starts_result : "null");
    if (starts_result) sentinel_string_free(starts_result);
    sentinel_query_free(starts_query);

    // Ends with
    sentinel_query_t* ends_query = sentinel_query_builder_new();
    CHECK_NULL(ends_query, "Failed to create query builder");
    CHECK_ERROR(sentinel_query_builder_filter_ends_with(ends_query, "department", "ing"));
    char* ends_result = sentinel_collection_query(users, ends_query);
    printf("Departments ending with 'ing': %s\n", ends_result ? ends_result : "null");
    if (ends_result) sentinel_string_free(ends_result);
    sentinel_query_free(ends_query);

    // Contains (already tested in previous example)
    sentinel_query_t* contains_query = sentinel_query_builder_new();
    CHECK_NULL(contains_query, "Failed to create query builder");
    CHECK_ERROR(sentinel_query_builder_filter_contains(contains_query, "department", "Engineer"));
    char* contains_result = sentinel_collection_query(users, contains_query);
    printf("Departments containing 'Engineer': %s\n", contains_result ? contains_result : "null");
    if (contains_result) sentinel_string_free(contains_result);
    sentinel_query_free(contains_query);

    // Test 3: In filter (value in array)
    printf("\n=== Test 3: In Filter (Value in Array) ===\n");
    sentinel_query_t* in_query = sentinel_query_builder_new();
    CHECK_NULL(in_query, "Failed to create query builder");
    CHECK_ERROR(sentinel_query_builder_filter_in(in_query, "city", "[\"New York\", \"Chicago\", \"Boston\"]"));
    char* in_result = sentinel_collection_query(users, in_query);
    printf("Cities in [New York, Chicago, Boston]: %s\n", in_result ? in_result : "null");
    if (in_result) sentinel_string_free(in_result);
    sentinel_query_free(in_query);

    // Test 4: Exists filter
    printf("\n=== Test 4: Exists Filter ===\n");

    // Field must exist
    sentinel_query_t* exists_query = sentinel_query_builder_new();
    CHECK_NULL(exists_query, "Failed to create query builder");
    CHECK_ERROR(sentinel_query_builder_filter_exists(exists_query, "level", 1));
    char* exists_result = sentinel_collection_query(users, exists_query);
    printf("Users with 'level' field: %s\n", exists_result ? exists_result : "null");
    if (exists_result) sentinel_string_free(exists_result);
    sentinel_query_free(exists_query);

    // Test 5: Complex query with multiple filters (AND logic)
    printf("\n=== Test 5: Complex AND Query ===\n");
    sentinel_query_t* complex_and = sentinel_query_builder_new();
    CHECK_NULL(complex_and, "Failed to create query builder");
    CHECK_ERROR(sentinel_query_builder_filter_equals(complex_and, "active", "true"));
    CHECK_ERROR(sentinel_query_builder_filter_greater_than(complex_and, "age", "25"));
    CHECK_ERROR(sentinel_query_builder_filter_less_than(complex_and, "age", "40"));
    CHECK_ERROR(sentinel_query_builder_filter_contains(complex_and, "department", "Engineer"));

    char* complex_result = sentinel_collection_query(users, complex_and);
    printf("Active engineers aged 26-39: %s\n", complex_result ? complex_result : "null");
    if (complex_result) sentinel_string_free(complex_result);
    sentinel_query_free(complex_and);

    // Test 6: OR operations (if supported)
    printf("\n=== Test 6: OR Operations ===\n");

    // Create two queries for OR operation
    sentinel_query_t* query_a = sentinel_query_builder_new();
    CHECK_NULL(query_a, "Failed to create query builder");
    CHECK_ERROR(sentinel_query_builder_filter_equals(query_a, "city", "\"New York\""));

    sentinel_query_t* query_b = sentinel_query_builder_new();
    CHECK_NULL(query_b, "Failed to create query builder");
    CHECK_ERROR(sentinel_query_builder_filter_equals(query_b, "city", "\"Chicago\""));

    // Combine with OR
    sentinel_query_t* or_query = sentinel_query_or(query_a, query_b);
    if (or_query) {
        char* or_result = sentinel_collection_query(users, or_query);
        printf("Users in New York OR Chicago: %s\n", or_result ? or_result : "null");
        if (or_result) sentinel_string_free(or_result);
        sentinel_query_free(or_query);
    } else {
        printf("OR operations not fully supported in this version\n");
    }

    sentinel_query_free(query_a);
    sentinel_query_free(query_b);

    // Test 7: Sorting and pagination with advanced filters
    printf("\n=== Test 7: Advanced Sorting & Pagination ===\n");
    sentinel_query_t* advanced_query = sentinel_query_builder_new();
    CHECK_NULL(advanced_query, "Failed to create query builder");
    CHECK_ERROR(sentinel_query_builder_filter_greater_or_equal(advanced_query, "score", "85"));
    CHECK_ERROR(sentinel_query_builder_filter_exists(advanced_query, "tags", 1));
    CHECK_ERROR(sentinel_query_builder_sort(advanced_query, "score", 1)); // Descending
    CHECK_ERROR(sentinel_query_builder_limit(advanced_query, 3)); // Top 3
    CHECK_ERROR(sentinel_query_builder_offset(advanced_query, 0)); // No offset

    char* advanced_result = sentinel_collection_query(users, advanced_query);
    printf("Top 3 users by score (>=85, has tags): %s\n", advanced_result ? advanced_result : "null");
    if (advanced_result) sentinel_string_free(advanced_result);
    sentinel_query_free(advanced_query);

    // Test 8: Count verification
    printf("\n=== Test 8: Final Statistics ===\n");
    uint32_t total_count = 0;
    CHECK_ERROR(sentinel_collection_count(users, &total_count));
    printf("Total users in system: %u\n", total_count);

    // Cleanup
    sentinel_collection_free(users);
    sentinel_store_free(store);

    printf("\n🎉 All advanced query and logical operations tests completed!\n");
    printf("✓ Comparison operators (>=, <=, >, <)\n");
    printf("✓ String matching (starts_with, ends_with, contains)\n");
    printf("✓ Array membership (in)\n");
    printf("✓ Field existence (exists)\n");
    printf("✓ Complex AND queries\n");
    printf("✓ OR operations (framework)\n");
    printf("✓ Advanced sorting and pagination\n");
    printf("✓ Combined filter operations\n");

    return 0;
}