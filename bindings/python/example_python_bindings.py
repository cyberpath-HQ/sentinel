#!/usr/bin/env python3
"""
Python example demonstrating Cyberpath Sentinel usage
"""

import asyncio
import tempfile
import os
import sys
from pathlib import Path

# Add the built extension to Python path
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "target" / "debug"))

import sentinel


async def main():
    """Main example function"""
    print("Cyberpath Sentinel Python Bindings Example")
    print("=" * 45)

    # Create a temporary directory for the database
    with tempfile.TemporaryDirectory() as temp_dir:
        db_path = os.path.join(temp_dir, "sentinel_example_db")
        print(f"Using database at: {db_path}\n")

        # 1. Create a store
        print("1. Creating Sentinel store...")
        store = await sentinel.Store.new(db_path)
        print("✓ Store created successfully\n")

        # 2. Create collections
        print("2. Creating collections...")
        users = await store.collection("users")
        products = await store.collection("products")
        orders = await store.collection("orders")
        print("✓ Collections created: users, products, orders\n")

        # 3. Insert sample data
        print("3. Inserting sample data...")

        # Users
        user_data = [
            {"id": "user_1", "name": "Alice Johnson", "email": "alice@example.com", "age": 30, "active": True},
            {"id": "user_2", "name": "Bob Smith", "email": "bob@example.com", "age": 25, "active": True},
            {"id": "user_3", "name": "Charlie Brown", "email": "charlie@example.com", "age": 35, "active": False},
        ]

        for user in user_data:
            await users.insert(user["id"], user)
        print(f"✓ Inserted {len(user_data)} users")

        # Products
        product_data = [
            {"id": "prod_1", "name": "Laptop", "price": 999.99, "category": "electronics", "stock": 50},
            {"id": "prod_2", "name": "Book", "price": 29.99, "category": "books", "stock": 100},
            {"id": "prod_3", "name": "Coffee Mug", "price": 12.99, "category": "kitchen", "stock": 75},
        ]

        for product in product_data:
            await products.insert(product["id"], product)
        print(f"✓ Inserted {len(product_data)} products")

        # Orders
        order_data = [
            {"id": "order_1", "user_id": "user_1", "product_id": "prod_1", "quantity": 1, "total": 999.99},
            {"id": "order_2", "user_id": "user_2", "product_id": "prod_2", "quantity": 2, "total": 59.98},
            {"id": "order_3", "user_id": "user_1", "product_id": "prod_3", "quantity": 3, "total": 38.97},
        ]

        for order in order_data:
            await orders.insert(order["id"], order)
        print(f"✓ Inserted {len(order_data)} orders\n")

        # 4. Basic CRUD operations
        print("4. Basic CRUD operations...")

        # Get a user
        alice = await users.get("user_1")
        print(f"✓ Retrieved user: {alice.data['name']} ({alice.data['email']})")

        # Update user
        alice.data["age"] = 31
        await users.update("user_1", alice.data)
        print("✓ Updated Alice's age to 31")

        # Upsert new user
        was_insert = await users.upsert("user_4", {"name": "Diana Wilson", "email": "diana@example.com", "age": 28, "active": True})
        print(f"✓ Upserted Diana (was_insert: {was_insert})")

        # Delete inactive user
        await users.delete("user_3")
        print("✓ Deleted inactive user Charlie\n")

        # 5. Query operations
        print("5. Query operations...")

        # Count documents
        user_count = await users.count()
        product_count = await products.count()
        order_count = await orders.count()
        print(f"✓ Document counts: {user_count} users, {product_count} products, {order_count} orders")

        # Query active users
        active_users = await users.query(filters=[("active", "eq", True)])
        print(f"✓ Found {len(active_users['documents'])} active users")

        # Query expensive products
        expensive_products = await users.query(
            filters=[("price", "gte", 100.0)],
            sort_by="price",
            sort_order="desc"
        )
        print(f"✓ Found {len(expensive_products['documents'])} expensive products")

        # Query with pagination
        paginated_orders = await orders.query(limit=2, offset=0)
        print(f"✓ Paginated query returned {len(paginated_orders['documents'])} orders (page 1)\n")

        # 6. Bulk operations
        print("6. Bulk operations...")

        # Bulk insert more products
        bulk_products = [
            ("prod_4", {"name": "Headphones", "price": 79.99, "category": "electronics", "stock": 30}),
            ("prod_5", {"name": "Notebook", "price": 4.99, "category": "stationery", "stock": 200}),
        ]

        await products.bulk_insert(bulk_products)
        print(f"✓ Bulk inserted {len(bulk_products)} products")

        # Get multiple products
        product_ids = ["prod_1", "prod_4", "prod_nonexistent"]
        retrieved_products = await products.get_many(product_ids)
        found_count = sum(1 for p in retrieved_products if p is not None)
        print(f"✓ Retrieved {found_count}/{len(product_ids)} requested products\n")

        # 7. Aggregation operations
        print("7. Aggregation operations...")

        # Count all orders
        total_orders = await orders.aggregate([], "count")
        print(f"✓ Total orders: {total_orders}")

        # Sum all order totals
        total_revenue = await orders.aggregate([], "sum")
        print(f"✓ Total revenue: ${total_revenue:.2f}")

        # Average product price
        avg_price = await products.aggregate([], "avg")
        print(f"✓ Average product price: ${avg_price:.2f}")

        # Count products by category
        electronics_count = await products.aggregate([("category", "eq", "electronics")], "count")
        print(f"✓ Electronics products: {electronics_count}\n")

        # 8. Collection management
        print("8. Collection management...")

        # List all collections
        collections = await store.list_collections()
        print(f"✓ Collections in store: {', '.join(collections)}")

        # Delete a collection
        await store.delete_collection("orders")
        collections_after = await store.list_collections()
        print(f"✓ Collections after deletion: {', '.join(collections_after)}\n")

        print("🎉 All Python binding examples completed successfully!")
        print("\nCyberpath Sentinel provides:")
        print("• Asynchronous document storage and retrieval")
        print("• Rich querying with filtering, sorting, and pagination")
        print("• Aggregation operations (count, sum, avg, min, max)")
        print("• Bulk operations for performance")
        print("• ACID-compliant transactions")
        print("• Full type safety with Python objects")


if __name__ == "__main__":
    asyncio.run(main())