#!/bin/bash
set -e

echo "🔄 Updating golden files for parity tests..."

cd tests/parity

# Generate new golden files
cargo run --bin generate_golden -- --output ./golden/

echo "✅ Golden files updated"