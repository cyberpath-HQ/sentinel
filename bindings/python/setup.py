#!/usr/bin/env python3

from setuptools import setup, Extension
from setuptools_rust import Binding, RustExtension
import os

# Get the absolute path to the sentinel-python crate
crate_path = os.path.join(os.path.dirname(__file__), "..", "crates", "sentinel-python")

setup(
    name="sentinel-dbms",
    version="2.0.1",
    description="Python bindings for Cyberpath Sentinel DBMS",
    long_description=open("README.md").read() if os.path.exists("README.md") else "",
    long_description_content_type="text/markdown",
    author="Emanuele (Ebalo) Balsamo",
    author_email="emanuele.balsamo@cyberpath-hq.com",
    url="https://github.com/cyberpath-HQ/sentinel",
    license="Apache-2.0",
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: Apache Software License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Programming Language :: Rust",
        "Topic :: Database",
        "Topic :: Software Development :: Libraries",
    ],
    keywords="database dbms document json filesystem",
    packages=["sentinel"],
    rust_extensions=[
        RustExtension(
            "sentinel.sentinel_python",
            path=crate_path,
            binding=Binding.PyO3,
            debug=False,
        )
    ],
    include_package_data=True,
    zip_safe=False,
    python_requires=">=3.8",
    install_requires=[
        "setuptools-rust>=1.5.0",
    ],
    extras_require={
        "dev": [
            "pytest>=7.0.0",
            "pytest-asyncio>=0.21.0",
            "black",
            "isort",
            "mypy",
        ],
    },
)