## 1. `tillerd-paths` crate

- [x] 1.1 Add a leaf crate `crates/paths` (package `tillerd-paths`) as a workspace member; deps limited to `std` + a home-directory helper; no other workspace crate (runtime-paths: dependency floor).
- [x] 1.2 Write failing tests for runtime-dir resolution: `TILLERD_DIR` override wins; default is `~/.tillerd`; an override-aware form prefers an explicit argument, then env, then default (runtime-paths: single runtime directory resolver).
- [x] 1.3 Implement `runtime_dir()` + `runtime_dir_or(Option<&str>)` (runtime-paths: single runtime directory resolver).
- [x] 1.4 Write failing tests then implement the pure path builders `daemon_socket_in`/`gate_socket_in`/`manifest_in`/`store_in` (file names defined only here) and their env-composed forms; assert all four share the runtime dir as parent (runtime-paths: runtime-layout path builders).
- [x] 1.5 Write failing tests then implement service-binary resolution (daemon/gate/notify): override-if-exists → `bin/<name>` or `target/{release,debug}/<name>` under cwd/ancestors → `~/.local/bin/<name>` → none; cover override-wins, override-skipped-when-missing, cargo-output-discovered, and none-when-absent (runtime-paths: service binary resolution by precedence).
- [x] 1.6 Define the governed `TILLERD_*` env-name constants (`ENV_TILLERD_DIR`, `ENV_DAEMON_BIN`, `ENV_GATE_BIN`, `ENV_NOTIFY_BIN`) and route the resolvers through them (runtime-paths: single source for the environment-variable surface).

## 2. Migrate `service-host`

- [x] 2.1 Replace `crates/service-host/src/paths.rs::resolve_base_dir` + `manifest_path` with `tillerd-paths` calls; delete the local impls; keep `service-host` tests green (runtime-paths: single resolver / path builders).

## 3. Migrate `process-launch`

- [x] 3.1 Replace `crates/process-launch/src/manifest.rs::tillerd_dir` + `manifest_path` with `tillerd-paths`; delete the locals; tests green.

## 4. Migrate `daemon-pty`

- [x] 4.1 Replace `apps/daemon-pty/src/manifest.rs::daemon_sock`/`manifest_path` and the `TILLERD_DIR` read in `main.rs` with `tillerd-paths`; delete the locals; tests green.

## 5. Migrate `gate` + `mcp-gateway`

- [x] 5.1 Replace the `TILLERD_DIR` reads and `gate.sock`/dir joins in `apps/gate/src/service.rs` and `apps/mcp-gateway/src/{config.rs,service.rs,gate_ipc.rs}` (`tillerd_dir`) with `tillerd-paths`; delete `mcp-gateway`'s local `tillerd_dir`; tests green.

## 6. Migrate `orchestrator`

- [x] 6.1 Replace `crates/orchestrator/src/surface/transport.rs::default_daemon_socket` and the `tillerd.db` path in `persistence/sqlite.rs::open_default` with `tillerd-paths` (`daemon_socket()`, `store_*`); tests green (runtime-paths: path builders; ADR-0023).

## 7. Migrate `desktop` host

- [x] 7.1 Replace `apps/desktop/src-tauri/src/paths.rs` (`tillerd_dir`, `daemon_sock`, `manifest_path`, `resolve_daemon_bin`/`gate`/`notify`) and the inline `dir.join("daemon.sock"|"gate.sock"|"daemon.json")` in `daemon_session.rs` + `orchestrator_host.rs` with `tillerd-paths`; delete the desktop resolvers; tests green (runtime-paths: all requirements).

## 8. Sweep for stragglers

- [x] 8.1 Grep the workspace for remaining direct `TILLERD_DIR`/`TILLERD_*_BIN` reads and hardcoded `daemon.sock`/`gate.sock`/`daemon.json`/`tillerd.db`/`.tillerd`/`.local/bin` outside `tillerd-paths` and route them through the crate (runtime-paths: single source).

## 9. Verification

- [x] 9.1 `tillerd-paths` exists with the runtime-dir resolver, the socket/manifest/store builders, the daemon/gate/notify resolvers (incl. the `target/release` fallback), and the `TILLERD_*` name constants, all unit-tested for precedence (acceptance 1).
- [x] 9.2 No crate other than `tillerd-paths` reads `TILLERD_DIR`/`TILLERD_*_BIN` directly or hardcodes the runtime file names/paths — confirmed by a workspace grep (acceptance 2). Production `src` is clean; remaining literals live only in test modules/fixtures (they exercise the resolver, not define it).
- [x] 9.3 The duplicate `tillerd_dir`/`resolve_base_dir`/`daemon_sock`/`manifest_path` impls are deleted (acceptance 3). `service-host`'s `Paths::manifest_path`/`socket_path` methods stay — per-tool `<name>.{json,sock}` host naming (gate.json/gateway.json), not a governed-name builder.
- [x] 9.4 `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `turbo build` pass workspace-wide (acceptance 4).
