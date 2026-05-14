#!/bin/bash
set -e

echo "📦 Installing MemFlow CLI..."

# Build CLI
cargo build -p memflow-cli

# Install to ~/.cargo/bin
cargo install -p memflow-cli --path .

echo "✅ CLI installed. Run 'memflow --help' to get started."