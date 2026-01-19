#!/bin/bash
# Build script for Cyberpath Sentinel C/C++ bindings

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "Building Cyberpath Sentinel C/C++ bindings..."
echo "Project root: $PROJECT_ROOT"
echo "Script dir: $SCRIPT_DIR"

# Build the Rust crate with C bindings
echo "Building Rust crate with C bindings..."
cd "$PROJECT_ROOT/language-interop"
cargo build --release -p sentinel-cxx

# Copy the generated header and library
HEADER_SRC="$PROJECT_ROOT/language-interop/crates/sentinel-cxx/target/release/sentinel-cxx.h"
HEADER_DST="$SCRIPT_DIR/cxx/include/sentinel/sentinel-cxx.h"
LIB_SRC="$PROJECT_ROOT/language-interop/target/release/libsentinel_cxx.so"
LIB_DST="$SCRIPT_DIR/cxx/lib/libsentinel_cxx.so"

if [ -f "$HEADER_SRC" ]; then
    echo "Copying generated header from $HEADER_SRC to $HEADER_DST"
    mkdir -p "$(dirname "$HEADER_DST")"
    cp "$HEADER_SRC" "$HEADER_DST"
else
    echo "Error: Generated header not found at $HEADER_SRC"
    exit 1
fi

if [ -f "$LIB_SRC" ]; then
    echo "Copying library from $LIB_SRC to $LIB_DST"
    mkdir -p "$(dirname "$LIB_DST")"
    cp "$LIB_SRC" "$LIB_DST"
else
    echo "Error: Library not found at $LIB_SRC"
    exit 1
fi

echo "C/C++ bindings built successfully!"
echo "Library: $LIB_DST"
echo "Header: $HEADER_DST"