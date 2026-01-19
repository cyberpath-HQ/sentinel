#!/usr/bin/env python3
"""
Python tests for Cyberpath Sentinel bindings
"""

import asyncio
import tempfile
import os
import sys
from pathlib import Path

# Add the built extension to Python path
sys.path.insert(0, str(Path(__file__).parent.parent.parent / "target" / "debug"))

import sentinel


async def test_basic_operations():
    """Test basic store and collection operations"""
    with tempfile.TemporaryDirectory() as temp_dir:
        print("Testing basic operations...")

        # Create store
        store = await sentinel.Store.new(os.path.join(temp_dir, "test_db"))
        assert store is not None, "Store creation failed"

        # Create collection
        users = await store.collection("users")
        assert users is not None, "Collection creation failed"

        # Insert document
        user_data = {
            "name": "Alice",
            "age": 30,
            "email": "alice@example.com"
        }
        await users.insert("alice", user_data)

        # Get document
        doc = await users.get("alice")
        assert doc is not None, "Document retrieval failed"
        assert doc.id == "alice", "Document ID mismatch"
        assert doc.data["name"] == "Alice", "Document data mismatch"

        # Update document
        updated_data = {
            "name": "Alice",
            "age": 31,
            "email": "alice@example.com"
        }
        await users.update("alice", updated_data)

        # Verify update
        doc = await users.get("alice")
        assert doc.data["age"] == 31, "Document update failed"

        # Upsert (should update existing)
        upsert_data = {
            "name": "Alice",
            "age": 32,
            "email": "alice@example.com",
            "active": True
        }
        was_insert = await users.upsert("alice", upsert_data)
        assert not was_insert, "Upsert should have been update"

        # Upsert (should insert new)
        was_insert = await users.upsert("bob", {"name": "Bob", "age": 25})
        assert was_insert, "Upsert should have been insert"

        # Count documents
        count = await users.count()
        assert count == 2, f"Expected 2 documents, got {count}"

        # Delete document
        await users.delete("bob")
        count = await users.count()
        assert count == 1, f"Expected 1 document after delete, got {count}"

        # List collections
        collections = await store.list_collections()
        assert "users" in collections, "Users collection not found in list"

        print("✓ Basic operations tests passed")


async def test_bulk_operations():
    """Test bulk insert and get_many operations"""
    with tempfile.TemporaryDirectory() as temp_dir:
        print("Testing bulk operations...")

        store = await sentinel.Store.new(os.path.join(temp_dir, "test_db"))
        products = await store.collection("products")

        # Bulk insert
        products_data = [
            ("laptop", {"name": "Laptop", "price": 999.99, "category": "electronics"}),
            ("book", {"name": "Programming Book", "price": 49.99, "category": "books"}),
            ("coffee", {"name": "Coffee Mug", "price": 12.99, "category": "kitchen"}),
        ]

        await products.bulk_insert(products_data)

        # Get multiple documents
        docs = await products.get_many(["laptop", "book", "nonexistent"])
        assert len(docs) == 3, "get_many should return 3 results"
        assert docs[0] is not None and docs[0].data["name"] == "Laptop"
        assert docs[1] is not None and docs[1].data["name"] == "Programming Book"
        assert docs[2] is None, "Nonexistent document should be None"

        print("✓ Bulk operations tests passed")


async def test_query_operations():
    """Test query operations"""
    with tempfile.TemporaryDirectory() as temp_dir:
        print("Testing query operations...")

        store = await sentinel.Store.new(os.path.join(temp_dir, "test_db"))
        employees = await store.collection("employees")

        # Insert test data
        employees_data = [
            {"name": "Alice", "age": 30, "department": "engineering", "salary": 80000},
            {"name": "Bob", "age": 25, "department": "engineering", "salary": 60000},
            {"name": "Charlie", "age": 35, "department": "marketing", "salary": 70000},
            {"name": "Diana", "age": 28, "department": "engineering", "salary": 75000},
        ]

        for i, emp in enumerate(employees_data):
            await employees.insert(f"emp_{i+1}", emp)

        # Query by department
        result = await employees.query(
            filters=[("department", "eq", "engineering")],
            sort_by="salary",
            sort_order="desc"
        )

        assert len(result["documents"]) == 3, "Should find 3 engineers"
        assert result["documents"][0].data["salary"] == 80000, "Should be sorted by salary desc"
        assert result["documents"][1].data["salary"] == 75000
        assert result["documents"][2].data["salary"] == 60000

        # Query with age range
        result = await employees.query(
            filters=[("age", "gte", 30)],
            limit=2
        )

        assert len(result["documents"]) == 2, "Should find 2 employees >= 30"
        assert result["total_count"] == 2

        print("✓ Query operations tests passed")


async def test_aggregation():
    """Test aggregation operations"""
    with tempfile.TemporaryDirectory() as temp_dir:
        print("Testing aggregation operations...")

        store = await sentinel.Store.new(os.path.join(temp_dir, "test_db"))
        sales = await store.collection("sales")

        # Insert sales data
        sales_data = [
            {"product": "A", "amount": 100},
            {"product": "A", "amount": 200},
            {"product": "B", "amount": 150},
            {"product": "B", "amount": 250},
        ]

        for i, sale in enumerate(sales_data):
            await sales.insert(f"sale_{i+1}", sale)

        # Count all sales
        count = await sales.aggregate([], "count")
        assert count == 4, f"Expected 4 sales, got {count}"

        # Sum all amounts
        total = await sales.aggregate([], "sum")
        assert total == 700, f"Expected total 700, got {total}"

        print("✓ Aggregation tests passed")


async def main():
    """Run all tests"""
    print("Running Python bindings tests...\n")

    try:
        await test_basic_operations()
        await test_bulk_operations()
        await test_query_operations()
        await test_aggregation()

        print("\n🎉 All Python binding tests passed!")

    except Exception as e:
        print(f"\n❌ Test failed: {e}")
        import traceback
        traceback.print_exc()
        return 1

    return 0


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)