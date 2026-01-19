import pytest
import asyncio
import tempfile
import os
import sys
from datetime import datetime

# Add the built extension to the path
workspace_target = os.path.join(os.path.dirname(__file__), '..', '..', '..', 'target', 'debug')
sys.path.insert(0, workspace_target)

try:
    import sentinel
except ImportError:
    pytest.skip("Python extension not built", allow_module_level=True)

@pytest.fixture
async def temp_store():
    """Create a temporary store for testing"""
    with tempfile.TemporaryDirectory() as temp_dir:
        store = await sentinel.Store.new(temp_dir)
        yield store

@pytest.fixture
async def temp_collection(temp_store):
    """Create a temporary collection for testing"""
    collection = await temp_store.collection("test_collection")
    yield collection

class TestStore:
    @pytest.mark.asyncio
    async def test_store_creation(self):
        """Test basic store creation"""
        with tempfile.TemporaryDirectory() as temp_dir:
            store = await sentinel.Store.new(temp_dir)
            assert store is not None

    @pytest.mark.asyncio
    async def test_store_creation_with_passphrase(self):
        """Test store creation with passphrase"""
        with tempfile.TemporaryDirectory() as temp_dir:
            store = await sentinel.Store.new(temp_dir, "test_passphrase")
            assert store is not None

    @pytest.mark.asyncio
    async def test_collection_creation(self, temp_store):
        """Test collection creation and retrieval"""
        collection = await temp_store.collection("users")
        assert collection is not None
        assert collection.name() == "users"

    @pytest.mark.asyncio
    async def test_list_collections_empty(self, temp_store):
        """Test listing collections when none exist"""
        collections = await temp_store.list_collections()
        assert isinstance(collections, list)
        assert len(collections) == 0

    @pytest.mark.asyncio
    async def test_list_collections_with_data(self, temp_store):
        """Test listing collections after creating some"""
        # Create collections
        await temp_store.collection("users")
        await temp_store.collection("products")
        await temp_store.collection("orders")

        collections = await temp_store.list_collections()
        assert isinstance(collections, list)
        assert len(collections) == 3
        assert "users" in collections
        assert "products" in collections
        assert "orders" in collections

    @pytest.mark.asyncio
    async def test_delete_collection(self, temp_store):
        """Test collection deletion"""
        # Create and verify collection exists
        await temp_store.collection("temp_collection")
        collections = await temp_store.list_collections()
        assert "temp_collection" in collections

        # Delete collection
        await temp_store.delete_collection("temp_collection")
        collections = await temp_store.list_collections()
        assert "temp_collection" not in collections

class TestCollection:
    @pytest.mark.asyncio
    async def test_insert_and_get_document(self, temp_collection):
        """Test inserting and retrieving a document"""
        test_data = {
            "name": "Alice",
            "age": 30,
            "email": "alice@example.com",
            "active": True,
            "tags": ["user", "admin"],
            "metadata": {
                "created_by": "system",
                "version": 1
            }
        }

        # Insert document
        await temp_collection.insert("user-123", test_data)

        # Retrieve document
        doc = await temp_collection.get("user-123")
        assert doc is not None
        assert doc.id == "user-123"
        assert doc.version == 1
        assert isinstance(doc.created_at, datetime)
        assert isinstance(doc.updated_at, datetime)
        assert doc.hash != ""
        assert doc.data == test_data

    @pytest.mark.asyncio
    async def test_get_nonexistent_document(self, temp_collection):
        """Test retrieving a document that doesn't exist"""
        doc = await temp_collection.get("nonexistent")
        assert doc is None

    @pytest.mark.asyncio
    async def test_delete_document(self, temp_collection):
        """Test soft deleting a document"""
        test_data = {"name": "Bob", "age": 25}

        # Insert and verify
        await temp_collection.insert("user-456", test_data)
        doc = await temp_collection.get("user-456")
        assert doc is not None

        # Delete
        await temp_collection.delete("user-456")
        doc = await temp_collection.get("user-456")
        assert doc is None

    @pytest.mark.asyncio
    async def test_count_documents(self, temp_collection):
        """Test counting documents in collection"""
        # Initially empty
        count = await temp_collection.count()
        assert count == 0

        # Add documents
        await temp_collection.insert("doc1", {"data": "test1"})
        await temp_collection.insert("doc2", {"data": "test2"})
        await temp_collection.insert("doc3", {"data": "test3"})

        count = await temp_collection.count()
        assert count == 3

    @pytest.mark.asyncio
    async def test_bulk_insert(self, temp_collection):
        """Test bulk inserting multiple documents"""
        documents = [
            ("bulk-1", {"name": "Bulk 1", "value": 1}),
            ("bulk-2", {"name": "Bulk 2", "value": 2}),
            ("bulk-3", {"name": "Bulk 3", "value": 3}),
        ]

        await temp_collection.bulk_insert(documents)

        # Verify all documents were inserted
        for doc_id, expected_data in documents:
            doc = await temp_collection.get(doc_id)
            assert doc is not None
            assert doc.data == expected_data

    @pytest.mark.asyncio
    async def test_document_overwrite(self, temp_collection):
        """Test overwriting an existing document"""
        original_data = {"name": "Original", "version": 1}
        updated_data = {"name": "Updated", "version": 2, "extra": "field"}

        # Insert original
        await temp_collection.insert("overwrite-test", original_data)
        doc = await temp_collection.get("overwrite-test")
        assert doc.data == original_data

        # Overwrite
        await temp_collection.insert("overwrite-test", updated_data)
        doc = await temp_collection.get("overwrite-test")
        assert doc.data == updated_data

class TestQueryBuilder:
    @pytest.mark.asyncio
    async def test_query_builder_creation(self):
        """Test creating a new query builder"""
        qb = sentinel.QueryBuilder()
        assert qb is not None

    @pytest.mark.asyncio
    async def test_query_builder_filter_equals(self):
        """Test adding equals filter"""
        qb = sentinel.QueryBuilder()
        qb = qb.filter("name", "equals", "Alice")
        assert qb is not None

    @pytest.mark.asyncio
    async def test_query_builder_sort(self):
        """Test adding sort"""
        qb = sentinel.QueryBuilder()
        qb = qb.sort("age", "ascending")
        qb = qb.sort("name", "descending")
        assert qb is not None

    @pytest.mark.asyncio
    async def test_query_builder_limit_offset(self):
        """Test limit and offset"""
        qb = sentinel.QueryBuilder()
        qb = qb.limit(10)
        qb = qb.offset(5)
        assert qb is not None

    @pytest.mark.asyncio
    async def test_query_builder_projection(self):
        """Test field projection"""
        qb = sentinel.QueryBuilder()
        qb = qb.projection(["name", "age", "email"])
        assert qb is not None

    @pytest.mark.asyncio
    async def test_query_execution(self, temp_collection):
        """Test executing a query"""
        # Insert test data
        await temp_collection.insert("query-1", {"name": "Alice", "age": 30, "city": "NYC"})
        await temp_collection.insert("query-2", {"name": "Bob", "age": 25, "city": "LA"})
        await temp_collection.insert("query-3", {"name": "Charlie", "age": 35, "city": "NYC"})

        # Create and execute query
        qb = sentinel.QueryBuilder()
        qb = qb.filter("city", "equals", "NYC")
        qb = qb.sort("age", "ascending")
        qb = qb.limit(2)

        result = await temp_collection.query(qb)
        assert result is not None
        # Note: QueryResult implementation is incomplete, will need to be extended

class TestDocument:
    @pytest.mark.asyncio
    async def test_document_properties(self, temp_collection):
        """Test document property access"""
        test_data = {
            "name": "Test Document",
            "count": 42,
            "active": True,
            "tags": ["tag1", "tag2"],
            "nested": {"key": "value"}
        }

        await temp_collection.insert("doc-props", test_data)
        doc = await temp_collection.get("doc-props")

        assert doc.id == "doc-props"
        assert doc.version == 1
        assert isinstance(doc.created_at, datetime)
        assert isinstance(doc.updated_at, datetime)
        assert doc.hash != ""
        assert doc.data == test_data

    @pytest.mark.asyncio
    async def test_document_timestamps(self, temp_collection):
        """Test document timestamp properties"""
        await temp_collection.insert("timestamp-test", {"data": "test"})

        doc = await temp_collection.get("timestamp-test")

        # Timestamps should be datetime objects
        assert isinstance(doc.created_at, datetime)
        assert isinstance(doc.updated_at, datetime)

        # Created and updated should be the same for new documents
        assert doc.created_at == doc.updated_at

class TestCryptoFunctions:
    @pytest.mark.asyncio
    async def test_hash_data(self):
        """Test hashing JSON data"""
        test_data = {"message": "Hello, World!", "number": 42}

        hash_result = await sentinel.hash_data(test_data)
        assert isinstance(hash_result, str)
        assert len(hash_result) > 0

        # Same data should produce same hash
        hash_result2 = await sentinel.hash_data(test_data)
        assert hash_result == hash_result2

    @pytest.mark.asyncio
    async def test_sign_and_verify(self):
        """Test signing and verifying data"""
        import os

        # Generate a random 32-byte key
        private_key = os.urandom(32)
        public_key = private_key  # In this crypto system, they might be the same

        test_hash = "abcdef1234567890" * 4  # 64 character hex string

        # Sign the hash
        signature = sentinel.sign_hash(test_hash, list(private_key))
        assert isinstance(signature, str)
        assert len(signature) > 0

        # Verify the signature
        is_valid = sentinel.verify_signature(test_hash, signature, list(public_key))
        assert is_valid is True

        # Test with wrong hash
        wrong_hash = "fedcba0987654321" * 4
        is_valid_wrong = sentinel.verify_signature(wrong_hash, signature, list(public_key))
        assert is_valid_wrong is False

class TestComplexDataTypes:
    @pytest.mark.asyncio
    async def test_nested_objects(self, temp_collection):
        """Test storing and retrieving nested objects"""
        nested_data = {
            "user": {
                "profile": {
                    "name": "Nested User",
                    "preferences": {
                        "theme": "dark",
                        "notifications": True
                    }
                },
                "accounts": [
                    {"type": "email", "address": "user@example.com"},
                    {"type": "phone", "number": "+1234567890"}
                ]
            },
            "metadata": {
                "version": 2,
                "tags": ["complex", "nested"]
            }
        }

        await temp_collection.insert("nested-doc", nested_data)
        doc = await temp_collection.get("nested-doc")

        assert doc.data == nested_data
        assert doc.data["user"]["profile"]["name"] == "Nested User"
        assert doc.data["user"]["accounts"][0]["type"] == "email"

    @pytest.mark.asyncio
    async def test_array_data(self, temp_collection):
        """Test storing and retrieving array data"""
        array_data = {
            "numbers": [1, 2, 3, 4, 5],
            "strings": ["a", "b", "c"],
            "booleans": [True, False, True],
            "mixed": [1, "two", True, {"nested": "object"}]
        }

        await temp_collection.insert("array-doc", array_data)
        doc = await temp_collection.get("array-doc")

        assert doc.data == array_data
        assert doc.data["numbers"] == [1, 2, 3, 4, 5]
        assert doc.data["mixed"][3]["nested"] == "object"

    @pytest.mark.asyncio
    async def test_null_values(self, temp_collection):
        """Test handling null values"""
        null_data = {
            "name": "Null Test",
            "optional_field": None,
            "empty_array": [],
            "empty_object": {}
        }

        await temp_collection.insert("null-doc", null_data)
        doc = await temp_collection.get("null-doc")

        assert doc.data == null_data
        assert doc.data["optional_field"] is None

if __name__ == "__main__":
    pytest.main([__file__])