#!/bin/bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

echo "Setting up development environment..."

# Setup .env -- seed from .env.example, rewriting every relative ./ path to an absolute path
# rooted at THIS worktree, so .tillerd and bin live beside it (each worktree is self-contained).
if [ -f .env ]; then
  echo ".env exists, skipping..."
else
  echo "Creating .env from .env.example..."
  sed "s|=\./|=$REPO_ROOT/|g" .env.example > .env
fi

# Create necessary directories
mkdir -p .tillerd bin

# Install dependencies
echo "Installing dependencies..."
bun install --frozen-lockfile

echo "✓ Development environment ready"
echo ""
echo "Next steps:"
echo "  bun run dev      — Start development server"
echo "  bun test         — Run tests"
echo "  bun run e2e      — Run e2e tests"
