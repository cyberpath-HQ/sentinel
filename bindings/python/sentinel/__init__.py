"""
Cyberpath Sentinel Python Bindings

A filesystem-backed document DBMS written in Rust with Python bindings.

Authors:
    Cyberpath <support@cyberpath-hq.com>
    Emanuele (Ebalo) Balsamo <emanuele.balsamo@cyberpath-hq.com>

Repository: https://github.com/cyberpath-HQ/sentinel
Documentation: https://sentinel.cyberpath-hq.com
"""

from sentinel_python import Store, Collection, Document, QueryBuilder, QueryResult, VerificationOptions
from sentinel_python import hash_data_py, sign_hash_py, verify_signature_py

__version__ = "2.0.1"
__author__ = "Cyberpath and Emanuele (Ebalo) Balsamo"
__email__ = "support@cyberpath-hq.com"
__license__ = "Apache-2.0"
__repository__ = "https://github.com/cyberpath-HQ/sentinel"
__documentation__ = "https://sentinel.cyberpath-hq.com"

__all__ = [
    "Store",
    "Collection",
    "Document",
    "QueryBuilder",
    "QueryResult",
    "VerificationOptions",
    "hash_data_py",
    "sign_hash_py",
    "verify_signature_py",
]