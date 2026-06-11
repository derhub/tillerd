## Why

Infrastructure refactor — no roadmap version; it hardens the consistency the 0.1.0 service
contract assumes.

Runtime-path resolution, runtime-layout constants, and service-binary discovery are duplicated
across the workspace and drift independently. Today there are **four** runtime-dir resolvers
(`apps/desktop/src-tauri/src/paths.rs::tillerd_dir`, `apps/mcp-gateway/src/config.rs::tillerd_dir`,
`crates/process-launch/src/manifest.rs::tillerd_dir`, `crates/service-host/src/paths.rs::resolve_base_dir`),
**three-plus** `daemon_sock`/`manifest_path` builders, and inline `dir.join("daemon.sock" |
"gate.sock" | "daemon.json")` in `daemon_session.rs` and `orchestrator_host.rs`. The `TILLERD_*`
environment-variable names are string literals in 11+ files. Each copy can — and does — diverge: the
desktop binary resolver lacked a `target/release` fallback while the build copied the binary
elsewhere, which is what kept breaking `dev` and `test` until it was patched in one place. A
per-crate `~/.tillerd` default can drift the same way. There is no single source of truth for "where
does tillerd put things and what are its env vars."

## What Changes

- **Introduce a `tillerd-paths` crate** — the lowest layer (no host or UI deps) and the single source
  of truth for the runtime layout and the `TILLERD_*` environment surface. It owns:
  - **Runtime dir resolution** — `TILLERD_DIR` → `~/.tillerd`, one implementation.
  - **Runtime-layout path builders** keyed off the runtime dir — daemon socket (`daemon.sock`), gate
    socket (`gate.sock`), daemon manifest (`daemon.json`), product store (`tillerd.db`).
  - **Service-binary resolution** for daemon / gate / notify, by the precedence `$TILLERD_*_BIN` →
    `bin/<name>` or `target/{release,debug}/<name>` under the cwd or an ancestor → `~/.local/bin/<name>`
    (the dev/CI auto-discovery currently living only in the desktop `paths.rs`).
  - **`TILLERD_*` env-var name constants** (the names, not the runtime auth-token values).
- **Migrate every owner to it. BREAKING (internal).** `service-host`, `process-launch`, `daemon-pty`,
  `gate`, `mcp-gateway`, `orchestrator` (`default_daemon_socket`, the `tillerd.db` path in
  `open_default`), and the `desktop` host depend on `tillerd-paths` and delete their local resolvers,
  builders, and hardcoded constants. Pre-v1; no compatibility shim.

A new dedicated crate is the right shape here even under the project's prefer-modules-over-new-crates
default: this is a shared foundation every service and the host depends on, so making it a module of
any one crate would invert the dependency graph. (Explicitly requested.)

## Capabilities

### New Capabilities

- `runtime-paths`: the single library that resolves the tillerd runtime directory, builds the
  socket/manifest/store paths under it, discovers the service binaries by a defined precedence, and
  is the one place the `TILLERD_*` environment-variable names are defined.

### Modified Capabilities

<!-- none — the migration of call sites is an implementation change; no existing capability's
     requirements change. The scattered resolvers were never spec-level behavior. -->

## Impact

- **New crate:** `crates/paths` (`tillerd-paths`), depended on inward by `service-host`,
  `process-launch`, `daemon-pty`, `gate`, `mcp-gateway`, `orchestrator`, and `apps/desktop/src-tauri`.
- **Deletions:** the four `tillerd_dir`/`resolve_base_dir` impls, the duplicate `daemon_sock`/
  `manifest_path` builders, the desktop binary resolvers, and inline socket/manifest `dir.join(...)`
  literals — replaced by `tillerd-paths` calls.
- **Behavior:** resolution becomes identical everywhere (notably the `target/{release,debug}` binary
  fallback now applies to every consumer, not just the desktop host).
- **ADRs:** honors ADR-0023 (one product store + `tillerd.db` path), ADR-0019/service-host (runtime
  dir + manifest), ADR-0008/0016 (daemon PTY socket), ADR-0018 (gate single socket), ADR-0022
  (orchestrator owns the backend). A new ADR records `tillerd-paths` as the single source of truth for
  the runtime layout and `TILLERD_*` surface, superseding the scattered resolvers.
- **Out of scope:** TS-side `TILLERD_*` reads in `apps/server` / `apps/ui` (a future generated or
  mirrored constants module); runtime auth-token *values* (`TILLERD_GATE_ADMIN_TOKEN`,
  `TILLERD_SESSION_ID`/`TOKEN` stay read where used — only their names may centralize); the
  launch-spec/config model (ADR-0021).
