#!/usr/bin/env python3

from setuptools import setup, Extension
from setuptools_rust import Binding, RustExtension
import os
import re

def get_version_from_cargo():
    """Read version from workspace Cargo.toml."""
    workspace_root = os.path.dirname(os.path.dirname(os.path.dirname(__file__)))
    cargo_path = os.path.join(workspace_root, "Cargo.toml")
    
    if os.path.exists(cargo_path):
        with open(cargo_path, "r") as f:
            content = f.read()
            # Look for version = "X.Y.Z" in [workspace.package] section
            match = re.search(r'^version\s*=\s*"(\d+\.\d+\.\d+)"', content, re.MULTILINE)
            if match:
                return match.group(1)
    
    # Fallback to a default version if not found
    return "2.0.1"

# Get the absolute path to the sentinel-python crate
crate_path = os.path.join(os.path.dirname(__file__), "..", "crates", "sentinel-python")

# Read README for long description
readme_path = os.path.join(os.path.dirname(__file__), "README.md")
long_description = open(readme_path).read() if os.path.exists(readme_path) else ""

# Get version from Cargo.toml to ensure sync with Rust library
version = get_version_from_cargo()

setup(
    name="sentinel-dbms",
    version=version,
    description="Python bindings for Cyberpath Sentinel DBMS",
    long_description=long_description,
    long_description_content_type="text/markdown",
    author="Cyberpath",
    author_email="support@cyberpath-hq.com",
    maintainer="Emanuele (Ebalo) Balsamo",
    maintainer_email="emanuele.balsamo@cyberpath-hq.com",
    url="https://github.com/cyberpath-HQ/sentinel",
    project_urls={
        "Bug Reports": "https://github.com/cyberpath-HQ/sentinel/issues",
        "Source": "https://github.com/cyberpath-HQ/sentinel",
        "Documentation": "https://sentinel.cyberpath-hq.com",
        "Changelog": "https://github.com/cyberpath-HQ/sentinel/blob/main/CHANGELOG.md",
    },
    license="Apache-2.0",
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: Apache Software License",
        "Operating System :: POSIX :: Linux",
        "Operating System :: MacOS :: MacOS X",
        "Operating System :: Microsoft :: Windows",
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
    keywords="database dbms document json filesystem rust async",
    packages=["sentinel"],
    rust_extensions=[
        RustExtension(
            "sentinel.sentinel",
            path=crate_path,
            binding=Binding.PyO3,
            debug=False,
        )
    ],
    include_package_data=True,
    package_data={
        "sentinel": ["py.typed", "*.pyi"],
    },
    zip_safe=False,
    python_requires=">=3.8",
    install_requires=[
        "setuptools-rust>=1.5.0",
        "pyo3>=0.20.0",
    ],
    extras_require={
        "dev": [
            "pytest>=7.0.0",
            "pytest-asyncio>=0.21.0",
            "build>=1.0.0",
            "twine>=4.0.0",
            "wheel>=0.42.0",
        ],
    },
)