#!/bin/bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Isolate E2E state from the user's runtime. Shared specs use one app; restart-owned specs get
# separate runtime directories that survive their own relaunches.
WORK="$(mktemp -d)"
export TILLERD_DIR="$WORK/current"

# WebdriverIO logs every command's RESULT at info level -- tens of thousands of lines that bury the
# test output. Keep only errors (bun:test reports pass/fail structurally; no log parsing).
export WDIO_LOG_LEVEL=error

# -- builds ------------------------------------------------------------------
# Build the services the app spawns and the UI bundle (frontendDist) once; both desktop builds
# embed that same frontend. Build the desktop binary via the tauri CLI (not raw cargo) so the app
# serves its embedded frontend -- a plain cargo/dev build loads the absent vite devUrl and renders
# about:blank.
bash tools/build-services.sh
bunx turbo run build --filter=@tillerd/ui

# Dev mode: the debug binary, fast to build, drives the full spec set.
(cd apps/desktop && bunx tauri build --debug --no-bundle --features webdriver)
DEV_BIN="$REPO_ROOT/target/debug/tillerd-desktop"
export TILLERD_DAEMON_BIN="$REPO_ROOT/bin/tillerd-daemon"
export TILLERD_GATE_BIN="$REPO_ROOT/bin/tillerd-gate"

# Bundled mode: the release binary with production-embedded assets. Built and boot-checked only when
# E2E_BUNDLED is set (CI does) so a local run stays on one build; isolated to the boot spec so a
# release-only packaging failure does not mask the rest of the suite (design D9).
BUNDLED_BIN=""
if [[ -n "${E2E_BUNDLED:-}" ]]; then
  (cd apps/desktop && bunx tauri build --no-bundle --features webdriver)
  BUNDLED_BIN="$REPO_ROOT/target/release/tillerd-desktop"
fi

# -- webdriver intermediary ----------------------------------------------------
owned_app_pids() {
  ps axeww -o pid=,stat=,command= | awk -v marker="TILLERD_DIR=$TILLERD_DIR" '
    $2 !~ /^Z/ && index($0, marker) && ($3 ~ /\/target\/(debug|release)\/tillerd-desktop$/ || $3 ~ /\/bin\/tillerd-(daemon|gate)$/) { print $1 }
  '
}

stop_app_processes() {
  local pid
  while read -r pid; do
    kill "$pid" 2>/dev/null || true
  done < <(owned_app_pids)
  for _ in $(seq 1 40); do
    [[ -z "$(owned_app_pids)" ]] && return 0
    sleep 0.25
  done
  while read -r pid; do
    kill -9 "$pid" 2>/dev/null || true
  done < <(owned_app_pids)
  for _ in $(seq 1 4); do
    [[ -z "$(owned_app_pids)" ]] && return 0
    sleep 0.25
  done
  echo "e2e CLEANUP FAILED: owned desktop/services still running after 10s." >&2
  return 1
}

WD=""

start_webdriver() {
  tauri-webdriver --port 4444 &
  WD=$!
  sleep 1
}

stop_webdriver() {
  if [[ -n "$WD" ]]; then
    kill "$WD" 2>/dev/null || true
    wait "$WD" 2>/dev/null || true
    WD=""
  fi
}

trap '
  stop_webdriver
  stop_app_processes || true
  rm -rf "$WORK"
' EXIT

# -- tests ---------------------------------------------------------------------
# Shared specs use one app from setup.ts. Own-launch specs run without the preload.
SETUP="$REPO_ROOT/tests/desktop-e2e/setup.ts"
BOOT="$REPO_ROOT/tests/desktop-e2e/boot.test.ts"
RESUME="$REPO_ROOT/tests/desktop-e2e/resume.test.ts"
RELOAD="$REPO_ROOT/tests/desktop-e2e/reload-deep-route.test.ts"
POINTERS="$REPO_ROOT/tests/desktop-e2e/view-pointers-restart.test.ts"
FOUNDATION="$REPO_ROOT/tests/desktop-e2e/foundation-integration.test.ts"
WORKBENCH_STATE="$REPO_ROOT/tests/desktop-e2e/workbench-state.test.ts"
EXPAND="$REPO_ROOT/tests/desktop-e2e/sidebar-expand-persist.test.ts"

# Own-launch specs: restart-resume, deep-route reload,
# the view-pointer restart, the foundation-integration journey (it reloads its own app), the
# workbench-layout restart (workbench chrome state has no URL, so only a real restart proves it),
# and the sidebar expand/collapse restart (sidebar tree state likewise has no URL).
OWN_LAUNCH_SPECS=(
  "$RESUME"
  "$RELOAD"
  "$POINTERS"
  "$FOUNDATION"
  "$WORKBENCH_STATE"
  "$EXPAND"
)
SHARED_SPECS=("$BOOT")
for spec in "$REPO_ROOT"/tests/desktop-e2e/*.test.ts; do
  case "$spec" in
    "$BOOT"|"$RESUME"|"$RELOAD"|"$POINTERS"|"$FOUNDATION"|"$WORKBENCH_STATE"|"$EXPAND") ;;
    *) SHARED_SPECS+=("$spec") ;;
  esac
done

run_tests() {
  local group="$1" binary="$2" runtime="$3" status
  shift 3
  mkdir -p "$runtime/.tillerd"
  # WebDriver inherits its environment once. Keep that path stable and retarget its runtime.
  ln -sfn "$runtime/.tillerd" "$TILLERD_DIR"

  echo "--- E2E: $group ---"
  set +e
  # The preload owns one WebDriver session and one app. Keep files sequential so their global
  # hooks cannot race that singleton lifecycle.
  TILLERD_DESKTOP_BIN="$binary" bun test --bail --max-concurrency 1 "$@"
  status=$?
  set -e
  if ! stop_app_processes; then
    status=1
  fi
  if (( status != 0 )); then
    echo "--- E2E TEST FAILED: $group ---" >&2
    tail -n 100 "$TILLERD_DIR"/logs/*.log 2>/dev/null >&2 || true
    return "$status"
  fi
}

start_webdriver

run_tests shared "$DEV_BIN" "$WORK/shared" --preload "$SETUP" "${SHARED_SPECS[@]}"

for spec in "${OWN_LAUNCH_SPECS[@]}"; do
  name="${spec##*/}"
  run_tests "own-launch: $name" "$DEV_BIN" "$WORK/${name%.test.ts}" "$spec"
done

# Bundled build: boot-to-ready only, using the same setup-owned lifecycle as the shared suite.
if [[ -n "$BUNDLED_BIN" ]]; then
  stop_webdriver
  stop_app_processes
  start_webdriver
  run_tests bundled-boot "$BUNDLED_BIN" "$WORK/bundled-boot" --preload "$SETUP" "$BOOT"
  stop_webdriver
fi
