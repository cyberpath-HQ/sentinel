"""
Cyberpath Sentinel Python Bindings

A filesystem-backed document DBMS written in Rust with Python bindings.
"""

from sentinel import Store, Collection, Document, QueryBuilder, QueryResult, VerificationOptions

__version__ = "2.0.1"
__author__ = "Emanuele (Ebalo) Balsamo"
__email__ = "emanuele.balsamo@cyberpath-hq.com"
__license__ = "Apache-2.0"

__all__ = ["Store", "Collection", "Document", "QueryBuilder", "QueryResult", "VerificationOptions"]