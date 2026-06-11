#!/bin/bash
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

# Build the supervised service binaries the orchestrator adopt-or-spawns. The
# binaries are resolved from target/<profile> and bin/ with no env (see the
# desktop host's resolve_daemon_bin/resolve_gate_bin). Used by dev, tests, CI.
profile="${TILLERD_BUILD_PROFILE:-release}"
flag=()
[ "$profile" = "release" ] && flag=(--release)

echo "Building tillerd services ($profile)..."
cargo build "${flag[@]}" --bin tillerd-daemon --bin tillerd-gate

mkdir -p bin
cp -f "target/$profile/tillerd-daemon" bin/tillerd-daemon
cp -f "target/$profile/tillerd-gate" bin/tillerd-gate

echo "✓ Services built: bin/tillerd-daemon, bin/tillerd-gate"
