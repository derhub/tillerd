#!/bin/bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" \
  || { echo "error: not a git repository" >&2; exit 1; }
[ -f "$REPO_ROOT/turbo.json" ] \
  || { echo "error: not the right repo" >&2; exit 1; }
cd "$REPO_ROOT"

# Stop daemon if running
if [ -f "${TILLERD_DIR:-$REPO_ROOT/.tillerd}/daemon.json" ]; then
  PID=$(jq -r '.pid' "${TILLERD_DIR:-$REPO_ROOT/.tillerd}/daemon.json" 2>/dev/null || true)
  [ -n "$PID" ] && kill "$PID" 2>/dev/null || true
fi

rm -rf "${TILLERD_DIR:-$REPO_ROOT/.tillerd}" bin/tillerd-daemon .env

find . \( -name node_modules -o -name .turbo -o -name target -o -name dist \
  -o -name .react-router -o -name test-results -o -name playwright-report -o -name build \
\) -prune -print | xargs --no-run-if-empty rm -rf

echo "✓ Done — run ./tools/setup-dev.sh to set up again"
