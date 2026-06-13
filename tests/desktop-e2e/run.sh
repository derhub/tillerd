#!/bin/bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Isolated runtime dir so the e2e never touches a real ~/.tillerd. Every app launch in this run
# (each test, and both restart launches of resume.test) inherits it, which is also what makes the
# resume-after-restart test a real restart against the same persisted workspace.
WORK="$(mktemp -d)"
export TILLERD_DIR="$WORK/.tillerd"
mkdir -p "$TILLERD_DIR"

# WebdriverIO logs every command's RESULT at info level — tens of thousands of lines that bury the
# test output. Keep only errors (bun:test reports pass/fail structurally; no log parsing).
export WDIO_LOG_LEVEL=error

# ── builds ──────────────────────────────────────────────────────────────────
# Build the services the app spawns and the UI bundle (frontendDist) once; both desktop builds
# embed that same frontend. Build the desktop binary via the tauri CLI (not raw cargo) so the app
# serves its embedded frontend — a plain cargo/dev build loads the absent vite devUrl and renders
# about:blank.
bash tools/build-services.sh
bunx turbo run build --filter=@tillerd/ui

# Dev mode: the debug binary, fast to build, drives the full spec set.
(cd apps/desktop && bunx tauri build --debug --no-bundle --features webdriver)
DEV_BIN="$REPO_ROOT/target/debug/tillerd-desktop"

# Bundled mode: the release binary with production-embedded assets. Built and boot-checked only when
# E2E_BUNDLED is set (CI does) so a local run stays on one build; isolated to the boot spec so a
# release-only packaging failure does not mask the rest of the suite (design D9).
BUNDLED_BIN=""
if [[ -n "${E2E_BUNDLED:-}" ]]; then
  (cd apps/desktop && bunx tauri build --no-bundle --features webdriver)
  BUNDLED_BIN="$REPO_ROOT/target/release/tillerd-desktop"
fi

# ── webdriver intermediary ────────────────────────────────────────────────────
# Starts the W3C WebDriver server (it launches the app per session) and reaps everything on exit:
# the intermediary, the app, AND the services the app spawned. The orchestrator leaves the
# gate/daemon running by design; the e2e must reap them so repeated runs do not accumulate orphans
# that exhaust resources and wedge a later boot.
tauri-webdriver --port 4444 &
WD=$!
trap '
  kill "$WD" 2>/dev/null || true
  pkill -f tillerd-desktop 2>/dev/null || true
  pkill -f "bin/tillerd-daemon" 2>/dev/null || true
  pkill -f "bin/tillerd-gate" 2>/dev/null || true
  rm -rf "$WORK"
' EXIT
sleep 1

# ── tests ─────────────────────────────────────────────────────────────────────
# `bun test` runs every *.test.ts sequentially in one process (no parallel app launches), reports
# pass/fail structurally, and exits non-zero on any failure — no log parsing. Each test defers its
# own app launch via the WebDriver session it opens. `--bail` stops at the first failure.

# Dev build: boot, full project/session create flows, resume-after-restart, surface isolation,
# terminal — the whole suite.
TILLERD_DESKTOP_BIN="$DEV_BIN" bun test --bail "$REPO_ROOT/tests/desktop-e2e"

# Bundled build: boot-to-ready only (isolated so a release-only packaging failure does not mask the
# rest of the suite — design D9).
if [[ -n "$BUNDLED_BIN" ]]; then
  TILLERD_DESKTOP_BIN="$BUNDLED_BIN" bun test --bail "$REPO_ROOT/tests/desktop-e2e/boot.test.ts"
fi
