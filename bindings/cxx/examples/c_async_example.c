#include "sentinel-cxx.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>  // For sleep

// Global counters for callback verification
static int store_created = 0;
static int collection_created = 0;
static int documents_inserted = 0;
static int documents_retrieved = 0;
static int errors_occurred = 0;

// Forward declarations for callback functions
void on_store_created(uint64_t task_id, sentinel_store_t* store, char* user_data);
void on_collection_created(uint64_t task_id, sentinel_collection_t* collection, char* user_data);
void on_document_inserted(uint64_t task_id, char* user_data);
void on_document_retrieved(uint64_t task_id, char* json_data, char* user_data);
void on_error(uint64_t task_id, const char* error_msg, char* user_data);

// Callback implementations
void on_store_created(uint64_t task_id, sentinel_store_t* store, char* user_data) {
    printf("✓ Store created asynchronously (task %llu, user_data: %s)\n", task_id, user_data);
    store_created = 1;

    // Now create a collection
    sentinel_collection_new_async(store, "async_test", on_collection_created, on_error, user_data);
}

void on_collection_created(uint64_t task_id, sentinel_collection_t* collection, char* user_data) {
    printf("✓ Collection created asynchronously (task %llu, user_data: %s)\n", task_id, user_data);
    collection_created = 1;

    // Insert some documents
    const char* docs[] = {
        "{\"name\": \"Alice\", \"role\": \"developer\"}",
        "{\"name\": \"Bob\", \"role\": \"manager\"}",
        "{\"name\": \"Charlie\", \"role\": \"designer\"}"
    };

    for (int i = 0; i < 3; i++) {
        char doc_id[32];
        sprintf(doc_id, "async_doc_%d", i + 1);
        sentinel_collection_insert_async(collection, doc_id, docs[i],
                                        on_document_inserted, on_error, user_data);
    }
}

void on_document_inserted(uint64_t task_id, char* user_data) {
    printf("✓ Document inserted asynchronously (task %llu, user_data: %s)\n", task_id, user_data);
    documents_inserted++;

    // After all documents are inserted, retrieve them
    if (documents_inserted == 3) {
        // Get the collection handle (simplified - in real code you'd store it)
        // For this example, we'll just demonstrate the pattern
        printf("All documents inserted, ready for retrieval operations\n");
    }
}

void on_document_retrieved(uint64_t task_id, char* json_data, char* user_data) {
    if (json_data) {
        printf("✓ Document retrieved asynchronously (task %llu): %s (user_data: %s)\n",
               task_id, json_data, user_data);
        sentinel_string_free(json_data);
    } else {
        printf("✓ Document not found (task %llu, user_data: %s)\n", task_id, user_data);
    }
    documents_retrieved++;
}

void on_error(uint64_t task_id, const char* error_msg, char* user_data) {
    printf("✗ Error in async operation (task %llu): %s (user_data: %s)\n",
           task_id, error_msg, user_data);
    errors_occurred++;
}

int main() {
    printf("Cyberpath Sentinel C Async API Example\n");
    printf("======================================\n\n");

    // Start async store creation
    printf("Starting async store creation...\n");
    printf("Callbacks: on_store_created=%p, on_error=%p\n", on_store_created, on_error);
    uint64_t store_task = sentinel_store_new_async("./async_test_db", NULL,
                                                   on_store_created, on_error, "async_demo");
    printf("Function returned: %llu\n", store_task);

    if (store_task == 0) {
        printf("Failed to start async store creation\n");
        const char* err = sentinel_get_last_error();
        if (err) {
            printf("Error: %s\n", err);
            sentinel_string_free((char*)err);
        }
        return 1;
    }

    // Wait for async operations to complete (in a real application, you'd use an event loop)
    printf("Waiting for async operations to complete...\n");
    int timeout = 30; // 30 seconds timeout
    while ((store_created == 0 || collection_created == 0 || documents_inserted < 3) && timeout > 0) {
        sleep(1);
        timeout--;

        // Check for pending tasks (in a real app, this would be event-driven)
        if (timeout % 5 == 0) {
            printf("Still waiting... (%d seconds remaining)\n", timeout);
        }
    }

    if (timeout == 0) {
        printf("Timeout waiting for async operations\n");
        return 1;
    }

    printf("\n✓ All async operations completed successfully!\n");
    printf("Summary:\n");
    printf("  - Stores created: %d\n", store_created);
    printf("  - Collections created: %d\n", collection_created);
    printf("  - Documents inserted: %d\n", documents_inserted);
    printf("  - Errors occurred: %d\n", errors_occurred);

    // Cleanup (in a real application, you'd properly track and free resources)
    printf("\nNote: In a production application, you would properly manage\n");
    printf("      resource cleanup and use an event loop instead of polling.\n");

    return 0;
}