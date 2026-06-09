## 1. e2e self-provisioning

- [x] 1.1 Add a daemon-lifecycle fixture in `tests/integration/fixtures/` that resolves the built `athing-daemon` binary, spawns it with a fresh temp `ATHING_DIR`, polls until `<dir>/daemon.sock` accepts a connection, and exposes the socket path
- [x] 1.2 Add teardown that kills the spawned daemon and removes the temp dir (afterAll), leaving no orphan process
- [x] 1.3 Wire the fixture into `tests/integration/daemon.test.ts` (beforeAll/afterAll) so `connect()` targets the spawned daemon's socket instead of a pre-running one
- [x] 1.4 Ensure the `e2e` turbo task makes the daemon binary available at the path the fixture resolves (build output or resolver), building on the existing `dependsOn @athing/daemon-pty#build`
- [x] 1.5 Run `bun run e2e` with `ATHING_DIR`/`ATHING_DAEMON_BIN` unset → daemon-protocol + engine tests pass; the engine `BinaryNotFound` negative test still passes

## 2. ui type-check

- [x] 2.1 Exclude `serve.ts` from `apps/ui/tsconfig.json` (`exclude`), since it is the runtime launcher outside the app's type config
- [x] 2.2 Confirm `ui#check-types` passes and no app code lost coverage

## 3. desktop lint

- [x] 3.1 Add `#![allow(dead_code)]` to `apps/desktop/src-tauri/src/orchestrator.rs` with a comment that the module is pending Tauri-command wiring (mirrors `gate_admin.rs`)
- [x] 3.2 Confirm `a-thing-desktop#lint` passes; record the orchestrator-wiring follow-up

## 4. Format gate (currently ungated)

- [x] 4.1 Add a non-mutating `format:check` script: `oxfmt --check .` + per-package `cargo fmt --check` (today `format` only writes, and `lint`/clippy does not catch rustfmt drift)
- [x] 4.2 Clear the pre-existing format debt the check surfaces (e.g. `apps/memorya/src/worker.rs`, `packages/platform-bun/src/process-launch.ts`, archived `openspec/**/design.md`) so `format:check` passes clean

## 5. Aggregate verification + CI

- [x] 5.1 Add a root `verify` script: `bun run format:check && turbo run check-types lint test e2e` (one pass/fail)
- [x] 5.2 Add a CI workflow (`.github/workflows/`) that installs deps and runs `bun run verify` on push and pull_request
- [x] 5.3 Document `bun run verify` as the pre-push gate in the README
- [x] 5.4 Run `bun run verify` on a clean checkout → format, type-check, lint, test, e2e all green
