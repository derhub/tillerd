#!/bin/bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

echo "Setting up development environment..."

# Setup .env with absolute paths
if [ ! -f .env ]; then
  echo "Creating .env from .env.example..."
  sed \
    -e "s|ATHING_DIR=\.\/\.athing|ATHING_DIR=$REPO_ROOT/.athing|" \
    -e "s|ATHING_DAEMON_BIN=\.\/bin\/athing-daemon|ATHING_DAEMON_BIN=$REPO_ROOT/bin/athing-daemon|" \
    .env.example > .env
else
  echo ".env exists, skipping..."
fi

# Create necessary directories
mkdir -p .athing bin

# Install dependencies
echo "Installing dependencies..."
bun install

echo "✓ Development environment ready"
echo ""
echo "Next steps:"
echo "  bun run dev      — Start development server"
echo "  bun test         — Run tests"
echo "  bun run e2e      — Run e2e tests"
