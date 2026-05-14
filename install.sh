#!/usr/bin/env bash
# MemFlow One-Click Install (Linux VPS / macOS)
set -e

RED='\033[0;31m'; GREEN='\033[0;32m'; CYAN='\033[0;36m'; NC='\033[0m'
echo -e "${CYAN}=== MemFlow Installer ===${NC}"

# Detect OS
OS="linux"
[[ "$(uname)" == "Darwin" ]] && OS="macos"

# Check prerequisites
command -v curl >/dev/null 2>&1 || { echo -e "${RED}curl required${NC}"; exit 1; }
command -v git >/dev/null 2>&1 || { echo -e "${RED}git required${NC}"; exit 1; }

# Install Rust
if ! command -v rustc &>/dev/null; then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Install Node.js (if needed)
if ! command -v node &>/dev/null; then
    echo "Installing Node.js..."
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
    apt-get install -y nodejs 2>/dev/null || brew install node 2>/dev/null
fi

# Clone repo
if [ ! -d "$HOME/memflow" ]; then
    git clone https://github.com/strueauto/memflow.git "$HOME/memflow"
fi
cd "$HOME/memflow"

# Build
echo -e "${CYAN}Building Rust workspace...${NC}"
cargo build --workspace --release 2>&1 | tail -3

echo -e "${CYAN}Setting up agent-service...${NC}"
cd agent-service && npm install && npm run build && cd ..

echo -e "${CYAN}Setting up web-ui...${NC}"
cd web-ui && npm install && npm run build && cd ..

# Create .env if missing
[ ! -f .env ] && cp .env.example .env 2>/dev/null || true

echo -e "${GREEN}=== MemFlow installed! ===${NC}"
echo -e "Start: ${CYAN}docker compose up -d${NC}"
echo -e "Or individually: ${CYAN}cargo run --package cli -- repl${NC}"
echo ""
echo "Edit .env to set your API keys, then run:"
echo "  docker compose up -d"
echo "  echo 'Your MemFlow agent is ready at http://localhost:3300'"
