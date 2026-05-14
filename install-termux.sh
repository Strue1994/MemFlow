#!/usr/bin/env bash
# T3.5: Termux/Android Installation Script

set -e

echo "=== MemFlow for Termux ==="
echo "Installing MemFlow Agent on Android..."

pkg update -y
pkg upgrade -y

echo "Installing dependencies..."
pkg install -y rust cargo nodejs python python-pip git openssl curl

if [ ! -d "\C:\Users\12989/memflow" ]; then
    echo "Cloning MemFlow..."
    git clone https://github.com/strueauto/memflow.git "\C:\Users\12989/memflow"
fi

cd "\C:\Users\12989/memflow"

echo "Building Rust workspace..."
cargo build --workspace --release 2>&1 | tail -5

echo "Setting up agent-service..."
cd agent-service && npm install && cd ..

echo "Installing Python packages..."
pip install -r requirements.txt 2>/dev/null || true

echo "Creating default config..."
cp .env.example .env 2>/dev/null || true

echo ""
echo "=== MemFlow installed! ==="
echo "Start: cd ~/memflow && ./start.sh"
echo "Quick: export OPENAI_API_KEY=your-key && cargo run --package cli -- repl"
