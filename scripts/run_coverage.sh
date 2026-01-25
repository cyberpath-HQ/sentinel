#!/bin/bash

# Script to run code coverage using grcov

set -e

echo "Installing grcov if not present..."
cargo install grcov || echo "grcov already installed"

echo "Creating profile directory..."
mkdir -p target/profraw

echo "Running tests with coverage instrumentation..."
RUSTFLAGS="-Cinstrument-coverage" LLVM_PROFILE_FILE="target/profraw/cyberpath-%p-%m.profraw" cargo test

echo "Generating coverage report..."
grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing -o ./target/coverage/html

echo "Coverage report generated at ./target/coverage/html/index.html"

# Optional: Generate lcov for CI
grcov . --binary-path ./target/debug/deps/ -s . -t lcov --branch --ignore-not-existing -o ./target/coverage/lcov.info

echo "LCOV report generated at ./target/coverage/lcov.info"