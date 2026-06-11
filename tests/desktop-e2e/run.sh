#!/bin/bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Isolated runtime dir so the e2e never touches a real ~/.tillerd. The launched app inherits it.
WORK="$(mktemp -d)"
export TILLERD_DIR="$WORK/.tillerd"
mkdir -p "$TILLERD_DIR"

# Build the services the app spawns, the UI bundle (frontendDist), and the desktop binary with
# the embedded WebDriver plugin.
# Build via the tauri CLI (not raw cargo) so the app serves its embedded frontend; a plain
# cargo/dev build loads the absent vite devUrl and renders about:blank.
bash tools/build-services.sh
bunx turbo run build --filter=@tillerd/ui
(cd apps/desktop && bunx tauri build --debug --no-bundle --features webdriver)
BIN="$REPO_ROOT/target/debug/tillerd-desktop"

# Start the WebDriver intermediary (it launches the app per session) and clean up on exit.
tauri-webdriver --port 4444 &
WD=$!
trap 'kill "$WD" 2>/dev/null || true; pkill -f tillerd-desktop 2>/dev/null || true; rm -rf "$WORK"' EXIT
sleep 1

TILLERD_DESKTOP_BIN="$BIN" bun "$REPO_ROOT/tests/desktop-e2e/terminal.smoke.ts"
