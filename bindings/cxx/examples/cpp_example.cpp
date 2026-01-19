#include <sentinel/sentinel.hpp>
#include <iostream>
#include <iomanip>
#include <string>
#include <vector>

int main() {
    std::cout << "Cyberpath Sentinel C++ API Example" << std::endl;
    std::cout << "===================================" << std::endl << std::endl;

    try {
        // Create a store
        std::cout << "Creating store at './example_db_cpp'..." << std::endl;
        sentinel::Store store("./example_db_cpp");

        // Create users collection
        std::cout << "Getting 'users' collection..." << std::endl;
        auto users = store.collection("users");

        // Insert some users
        std::cout << "Inserting users..." << std::endl;

        users->insert("user1", R"(
{
    "name": "Alice Johnson",
    "email": "alice@example.com",
    "age": 28,
    "active": true,
    "tags": ["developer", "admin"]
})");

        users->insert("user2", R"(
{
    "name": "Bob Smith",
    "email": "bob@example.com",
    "age": 34,
    "active": false,
    "department": "sales"
})");

        users->insert("user3", R"(
{
    "name": "Charlie Brown",
    "email": "charlie@example.com",
    "age": 25,
    "active": true,
    "skills": ["C++", "Rust", "Python"]
})");

        // Get document count
        size_t count = users->count();
        std::cout << "Users collection now has " << count << " documents" << std::endl << std::endl;

        // Retrieve and display users
        std::cout << "Retrieving users:" << std::endl;
        std::vector<std::string> user_ids = {"user1", "user2", "user3"};

        for (const auto& user_id : user_ids) {
            try {
                std::string user_data = users->get(user_id);
                std::cout << "  " << user_id << ": " << user_data << std::endl;
            } catch (const sentinel::SentinelException& e) {
                std::cout << "  " << user_id << ": Error - " << e.what() << std::endl;
            }
        }
        std::cout << std::endl;

        // Update a user
        std::cout << "Updating user1..." << std::endl;
        users->update("user1", R"(
{
    "name": "Alice Cooper",
    "email": "alice@example.com",
    "age": 29,
    "active": true,
    "tags": ["senior-developer", "admin"],
    "last_login": "2024-01-15T10:30:00Z"
})");

        // Verify update
        std::string updated_user = users->get("user1");
        std::cout << "Updated user1: " << updated_user << std::endl << std::endl;

        // Upsert operations
        std::cout << "Upsert operations:" << std::endl;

        // Upsert existing user (should update)
        bool was_insert = users->upsert("user2", R"(
{
    "name": "Bob Wilson",
    "email": "bob@example.com",
    "age": 35,
    "active": true,
    "department": "engineering",
    "projects": ["sentinel", "crypto"]
})");
        std::cout << "  Upsert user2 (existing): " << (was_insert ? "inserted" : "updated") << std::endl;

        // Upsert new user (should insert)
        was_insert = users->upsert("user4", R"(
{
    "name": "Diana Prince",
    "email": "diana@example.com",
    "age": 30,
    "active": true,
    "role": "manager",
    "team": ["alice", "bob", "charlie"]
})");
        std::cout << "  Upsert user4 (new): " << (was_insert ? "inserted" : "updated") << std::endl;

        // Get final count
        count = users->count();
        std::cout << "Users collection now has " << count << " documents" << std::endl << std::endl;

        // List all collections
        std::cout << "Listing all collections:" << std::endl;
        auto collections = store.list_collections();
        for (const auto& name : collections) {
            std::cout << "  " << name << std::endl;
        }
        std::cout << std::endl;

        // Create another collection for orders
        std::cout << "Creating 'orders' collection..." << std::endl;
        auto orders = store.collection("orders");

        // Add some orders
        orders->insert("order1", R"(
{
    "user_id": "user1",
    "items": [
        {"product": "Laptop", "quantity": 1, "price": 999.99},
        {"product": "Mouse", "quantity": 2, "price": 25.50}
    ],
    "total": 1050.99,
    "status": "completed",
    "created_at": "2024-01-15T14:30:00Z"
})");

        orders->insert("order2", R"(
{
    "user_id": "user3",
    "items": [
        {"product": "Book", "quantity": 3, "price": 19.99}
    ],
    "total": 59.97,
    "status": "pending",
    "created_at": "2024-01-16T09:15:00Z"
})");

        // Show orders
        std::cout << "Orders:" << std::endl;
        std::string order1 = orders->get("order1");
        std::cout << "  order1: " << order1 << std::endl;

        std::string order2 = orders->get("order2");
        std::cout << "  order2: " << order2 << std::endl;
        std::cout << std::endl;

        // List collections again
        std::cout << "All collections:" << std::endl;
        collections = store.list_collections();
        for (const auto& name : collections) {
            std::cout << "  " << name << std::endl;
        }
        std::cout << std::endl;

        // Delete operations
        std::cout << "Deleting operations:" << std::endl;

        // Delete a user
        users->delete_document("user3");
        std::cout << "  Deleted user3" << std::endl;

        // Delete an order
        orders->delete_document("order2");
        std::cout << "  Deleted order2" << std::endl;

        // Show final counts
        size_t users_count = users->count();
        size_t orders_count = orders->count();
        std::cout << "Final counts - Users: " << users_count << ", Orders: " << orders_count << std::endl << std::endl;

        // Demonstrate error handling
        std::cout << "Error handling demonstration:" << std::endl;
        try {
            users->get("nonexistent_user");
        } catch (const sentinel::SentinelException& e) {
            std::cout << "  Expected error when getting nonexistent user: " << e.what() << std::endl;
        }

        try {
            users->delete_document("nonexistent_user");
        } catch (const sentinel::SentinelException& e) {
            std::cout << "  Expected error when deleting nonexistent user: " << e.what() << std::endl;
        }

        std::cout << std::endl;

        // Clean up - delete orders collection
        std::cout << "Cleaning up - deleting orders collection..." << std::endl;
        store.delete_collection("orders");

        // Final collection list
        std::cout << "Final collections:" << std::endl;
        collections = store.list_collections();
        for (const auto& name : collections) {
            std::cout << "  " << name << std::endl;
        }

        std::cout << std::endl << "C++ example completed successfully!" << std::endl;

    } catch (const sentinel::SentinelException& e) {
        std::cerr << "Unexpected error: " << e.what() << std::endl;
        return 1;
    } catch (const std::exception& e) {
        std::cerr << "Standard exception: " << e.what() << std::endl;
        return 1;
    }

    return 0;
}