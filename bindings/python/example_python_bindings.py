#!/usr/bin/env python3
"""
Example usage of the Sentinel Python bindings.

This script demonstrates the basic usage of the Sentinel database
through its Python bindings.
"""

import asyncio
import tempfile
import os

# Add the built extension to the path
workspace_target = os.path.join(os.path.dirname(__file__), '..', '..', 'target', 'debug')
if workspace_target not in os.path.sys.path:
    os.sys.path.insert(0, workspace_target)

import sentinel_python as sentinel


async def main():
    print("🚀 Sentinel Python Bindings Example")
    print("=" * 50)

    # Create a temporary directory for our database
    with tempfile.TemporaryDirectory() as temp_dir:
        print(f"📁 Using temporary directory: {temp_dir}")

        # Create a new Sentinel store
        print("\n🏪 Creating Sentinel store...")
        store = await sentinel.Store.new(temp_dir, "example_passphrase")
        print("✅ Store created successfully")

        # Create collections
        print("\n📚 Creating collections...")
        users = await store.collection("users")
        products = await store.collection("products")
        orders = await store.collection("orders")
        print("✅ Collections created: users, products, orders")

        # Insert some user data
        print("\n👤 Inserting user documents...")
        await users.insert("user-001", {
            "name": "Alice Johnson",
            "email": "alice@example.com",
            "age": 28,
            "active": True,
            "tags": ["customer", "premium"],
            "preferences": {
                "theme": "dark",
                "notifications": True,
                "language": "en"
            }
        })

        await users.insert("user-002", {
            "name": "Bob Smith",
            "email": "bob@example.com",
            "age": 35,
            "active": True,
            "tags": ["customer"],
            "preferences": {
                "theme": "light",
                "notifications": False,
                "language": "en"
            }
        })

        await users.insert("user-003", {
            "name": "Charlie Brown",
            "email": "charlie@example.com",
            "age": 42,
            "active": False,
            "tags": ["customer", "inactive"],
            "preferences": {
                "theme": "light",
                "notifications": True,
                "language": "es"
            }
        })
        print("✅ Users inserted successfully")

        # Insert some product data
        print("\n📦 Inserting product documents...")
        await products.insert("product-001", {
            "name": "Wireless Headphones",
            "price": 199.99,
            "category": "Electronics",
            "in_stock": True,
            "tags": ["audio", "wireless", "premium"],
            "specifications": {
                "battery_life": "30 hours",
                "connectivity": "Bluetooth 5.0",
                "weight": "250g"
            }
        })

        await products.insert("product-002", {
            "name": "Ergonomic Office Chair",
            "price": 349.99,
            "category": "Furniture",
            "in_stock": True,
            "tags": ["office", "ergonomic", "comfort"],
            "specifications": {
                "material": "Mesh",
                "adjustable": True,
                "weight_limit": "300 lbs"
            }
        })
        print("✅ Products inserted successfully")

        # Insert some order data
        print("\n🛒 Inserting order documents...")
        await orders.insert("order-001", {
            "user_id": "user-001",
            "product_ids": ["product-001", "product-002"],
            "total_amount": 549.98,
            "status": "completed",
            "created_at": "2024-01-15T10:30:00Z",
            "shipping_address": {
                "street": "123 Main St",
                "city": "Anytown",
                "state": "CA",
                "zip": "12345"
            }
        })

        await orders.insert("order-002", {
            "user_id": "user-002",
            "product_ids": ["product-001"],
            "total_amount": 199.99,
            "status": "pending",
            "created_at": "2024-01-16T14:20:00Z",
            "shipping_address": {
                "street": "456 Oak Ave",
                "city": "Somewhere",
                "state": "NY",
                "zip": "67890"
            }
        })
        print("✅ Orders inserted successfully")

        # Query operations
        print("\n🔍 Performing queries...")

        # Count documents in each collection
        users_count = await users.count()
        products_count = await products.count()
        orders_count = await orders.count()
        print(f"📊 Collection counts - Users: {users_count}, Products: {products_count}, Orders: {orders_count}")

        # Get a specific user
        print("\n👤 Retrieving user by ID...")
        user = await users.get("user-001")
        if user:
            print(f"✅ Found user: {user.data['name']} ({user.data['email']})")
            print(f"   Created: {user.created_at}")
            print(f"   Version: {user.version}")
            print(f"   Hash: {user.hash[:16]}...")

        # Query with filters
        print("\n🔍 Querying active users...")
        qb = sentinel.QueryBuilder()
        qb = qb.filter("active", "equals", True)
        qb = qb.sort("age", "ascending")

        result = await users.query(qb)
        documents = result.documents()
        print(f"✅ Found {len(documents)} active users (sorted by age):")
        for doc in documents:
            print(f"   - {doc.data['name']} ({doc.data['age']} years old)")

        # Query products by category
        print("\n📦 Querying electronics products...")
        qb = sentinel.QueryBuilder()
        qb = qb.filter("category", "equals", "Electronics")
        qb = qb.limit(5)

        result = await products.query(qb)
        documents = result.documents()
        print(f"✅ Found {len(documents)} electronics products:")
        for doc in documents:
            print(f"   - {doc.data['name']}: ${doc.data['price']}")

        # Get multiple documents at once
        print("\n📑 Bulk retrieving users...")
        user_ids = ["user-001", "user-002", "user-999"]  # Last one doesn't exist
        users_bulk = await users.get_many(user_ids)
        print(f"✅ Bulk retrieved {len(users_bulk)} users:")
        for i, user_doc in enumerate(users_bulk):
            if user_doc:
                print(f"   [{i}] {user_doc.data['name']}")
            else:
                print(f"   [{i}] Not found")

        # Update a document
        print("\n📝 Updating user document...")
        original_user = await users.get("user-001")
        original_age = original_user.data['age'] if original_user else None

        await users.update("user-001", {
            "name": "Alice Johnson",
            "email": "alice@example.com",
            "age": 29,  # Birthday!
            "active": True,
            "tags": ["customer", "premium", "birthday"],
            "preferences": {
                "theme": "dark",
                "notifications": True,
                "language": "en"
            }
        })

        updated_user = await users.get("user-001")
        if updated_user:
            print(f"✅ Updated user age from {original_age} to {updated_user.data['age']}")
            print(f"   Updated timestamp: {updated_user.updated_at}")

        # Upsert operation
        print("\n🔄 Upserting document...")
        was_insert = await users.upsert("user-new", {
            "name": "New User",
            "email": "new@example.com",
            "age": 25,
            "active": True,
            "tags": ["customer"]
        })
        print(f"✅ Upsert result: {'INSERT' if was_insert else 'UPDATE'}")

        # Test aggregation
        print("\n📊 Aggregating user data...")
        # Count active users
        active_count = users.aggregate(
            [("active", "equals", True)],
            "count"
        )
        print(f"✅ Active users count: {active_count}")

        # List all collections
        print("\n📚 All collections in store:")
        collections = await store.list_collections()
        for collection_name in collections:
            print(f"   - {collection_name}")

        # Clean up - delete a collection
        print("\n🗑️  Deleting 'orders' collection...")
        await store.delete_collection("orders")
        collections_after = await store.list_collections()
        print(f"✅ Collections after deletion: {collections_after}")

        # Crypto operations
        print("\n🔐 Testing crypto functions...")
        test_data = {"message": "Hello from Sentinel!", "timestamp": "2024-01-19"}
        hash_result = await sentinel.hash_data_py(test_data)
        print(f"✅ Data hash generated: {hash_result[:32]}...")
        
        # Note: In production, key generation and signing would use proper key management
        # For this example, we just verify the hash function works
        print("✅ Crypto functions available (sign_hash, verify_signature require proper key handling)")

        print("\n🎉 Example completed successfully!")
        print("=" * 50)


if __name__ == "__main__":
    asyncio.run(main())