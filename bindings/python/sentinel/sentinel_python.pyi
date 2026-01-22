"""
Stub file for sentinel_python module.

This file provides type hints for the Cyberpath Sentinel Python bindings.
"""
from __future__ import annotations
from typing import Any, AsyncIterator, Dict, List, Optional, Tuple, Union
import datetime

class Store:
    """Represents a Sentinel document store."""

    @staticmethod
    async def new(path: str, passphrase: Optional[str] = None) -> Store:
        """Create a new Sentinel store."""
        ...

    async def collection(self, name: str) -> Collection:
        """Get or create a collection with the specified name."""
        ...

    async def delete_collection(self, name: str) -> None:
        """Delete a collection and all its documents."""
        ...

    async def list_collections(self) -> List[str]:
        """List all collections in the store."""
        ...

class Collection:
    """Represents a document collection."""

    @property
    def name(self) -> str:
        """Get the collection name."""
        ...

    async def insert(self, id: str, data: Dict[str, Any]) -> None:
        """Insert a new document or overwrite an existing one."""
        ...

    async def get(self, id: str) -> Optional[Document]:
        """Retrieve a document by its ID."""
        ...

    async def get_with_verification(
        self, id: str, options: VerificationOptions
    ) -> Optional[Document]:
        """Retrieve a document with custom verification options."""
        ...

    async def update(self, id: str, data: Dict[str, Any]) -> None:
        """Update an existing document."""
        ...

    async def upsert(self, id: str, data: Dict[str, Any]) -> bool:
        """Insert or update a document."""
        ...

    async def delete(self, id: str) -> None:
        """Delete a document (soft delete)."""
        ...

    async def get_many(self, ids: List[str]) -> List[Optional[Document]]:
        """Get multiple documents by IDs."""
        ...

    async def count(self) -> int:
        """Count the total number of documents in the collection."""
        ...

    async def bulk_insert(
        self, documents: List[Tuple[str, Dict[str, Any]]]
    ) -> None:
        """Insert multiple documents in a single operation."""
        ...

    async def query(self, query: QueryBuilder) -> QueryResult:
        """Execute a query against the collection."""
        ...

    async def query_with_verification(
        self, query: QueryBuilder, options: VerificationOptions
    ) -> QueryResult:
        """Execute a query with custom verification options."""
        ...

    async def aggregate(
        self,
        filters: List[Tuple[str, str, Any]],
        aggregation: str,
    ) -> Any:
        """Aggregate documents."""
        ...

class Document:
    """Represents a Sentinel document."""

    @property
    def id(self) -> str:
        """Get the document ID."""
        ...

    @property
    def version(self) -> int:
        """Get the document version."""
        ...

    @property
    def created_at(self) -> datetime.datetime:
        """Get the document creation timestamp."""
        ...

    @property
    def updated_at(self) -> datetime.datetime:
        """Get the document last update timestamp."""
        ...

    @property
    def hash(self) -> str:
        """Get the document hash."""
        ...

    @property
    def signature(self) -> str:
        """Get the document signature."""
        ...

    @property
    def data(self) -> Dict[str, Any]:
        """Get the document data."""
        ...

class QueryBuilder:
    """Builder for constructing queries."""

    @staticmethod
    def new() -> QueryBuilder:
        """Create a new empty query builder."""
        ...

    def filter(
        self, field: str, operator: str, value: Any
    ) -> QueryBuilder:
        """Add a filter condition to the query."""
        ...

    def sort(self, field: str, order: str) -> QueryBuilder:
        """Add a sort condition to the query."""
        ...

    def limit(self, limit: int) -> QueryBuilder:
        """Set the maximum number of results."""
        ...

    def offset(self, offset: int) -> QueryBuilder:
        """Set the number of results to skip."""
        ...

    def projection(self, fields: List[str]) -> QueryBuilder:
        """Set the fields to include in results."""
        ...

class QueryResult:
    """Represents the result of a query."""

    @property
    def documents(self) -> List[Document]:
        """Get the matching documents."""
        ...

    @property
    def total_count(self) -> Optional[int]:
        """Get the total count of matching documents."""
        ...

    @property
    def execution_time(self) -> float:
        """Get the execution time of the query in seconds."""
        ...

class VerificationOptions:
    """Options for controlling verification behavior when reading documents."""

    @staticmethod
    def default() -> VerificationOptions:
        """Create default verification options (strict verification)."""
        ...

    @staticmethod
    def warn() -> VerificationOptions:
        """Create verification options that warn instead of failing."""
        ...

    @staticmethod
    def silent() -> VerificationOptions:
        """Create verification options that skip verification."""
        ...

async def hash_data_py(data: Dict[str, Any]) -> str:
    """Hash data using the configured algorithm."""
    ...

async def sign_hash_py(hash: str, private_key_bytes: List[int]) -> str:
    """Sign a hash using the configured algorithm."""
    ...

async def verify_signature_py(
    hash: str, signature: str, public_key_bytes: List[int]
) -> bool:
    """Verify a signature using the configured algorithm."""
    ...
