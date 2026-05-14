#!/bin/bash
set -e

echo "🚀 Starting MemFlow in development mode..."

# Start dependency services
docker-compose -f docker-compose.dev.yml up -d

# Start executor (hot reload)
cargo run -p executor &

# Start agent service
cd agent-service && npm run dev &

# Start web UI
cd web-ui && npm run dev &

wait