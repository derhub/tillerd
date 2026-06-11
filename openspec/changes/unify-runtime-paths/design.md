## Context

The tillerd runtime layout — where the daemon socket, gate socket, manifest, and product store live,
and how the service binaries are found — is resolved in many places that drift apart. Four crates
each define a runtime-dir resolver; several rebuild the same socket/manifest paths; the `TILLERD_*`
variable names are string literals across 11+ files; the binary resolver's `target/release` fallback
exists only in the desktop host. The recent dev/test breakage (binaries unresolvable without env) was
a direct symptom: a fix in one resolver did not reach the others.

The in-force decisions this must stay coherent with: ADR-0023 (one product store; `tillerd.db` under
the runtime dir), ADR-0019 + the service-host model (runtime dir, manifest), ADR-0008/0016 (daemon
PTY socket), ADR-0018 (gate single socket), ADR-0022 (orchestrator owns the backend). None of them
change; this change centralizes their shared assumption about the runtime layout.

## Goals / Non-Goals

**Goals:**

- One crate, `tillerd-paths`, as the lowest layer (depends only on the standard library plus a
  home-directory helper), owning runtime-dir resolution, the layout path builders, service-binary
  resolution, and the `TILLERD_*` name constants.
- Every current owner migrated to it; the duplicate resolvers, builders, and literals deleted.
- Identical resolution everywhere — in particular the `target/{release,debug}` binary fallback
  applies to all consumers.

**Non-Goals:**

- TS-side `TILLERD_*` reads (`apps/server`, `apps/ui`) — a future mirrored/generated constants module.
- Runtime auth-token *values* (`TILLERD_GATE_ADMIN_TOKEN`, `TILLERD_SESSION_ID`/`TOKEN`) — they stay
  read where used; only their names may centralize later.
- The launch-spec/config model (ADR-0021); changing any path or file name; a config file format.

## Decisions

**Separate "resolve the dir" from "build paths under a dir".** The builders are pure functions of a
directory (`daemon_socket_in(dir)`, `manifest_in(dir)`, `gate_socket_in(dir)`, `store_in(dir)`), and
the env-reading entry points (`runtime_dir()`, `daemon_socket()`, …) compose them with the resolved
dir. _Why:_ `service-host` resolves its dir from a CLI override (`resolve_base_dir(Option<&str>)`),
while most callers read the env — pure builders serve both without forcing an env read. Provide
`runtime_dir()` (env) and `runtime_dir_or(override: Option<&str>)` (override-then-env-then-default).

**`tillerd-paths` is the dependency floor.** It depends only on `std` + a home-dir helper and on no
other workspace crate, so `service-host`, `process-launch`, `contracts`-adjacent crates, and the
host can all depend on it without cycles. _Alternative:_ put it in `service-host` — rejected:
`daemon-pty`/`gate`/`mcp-gateway` would then depend on `service-host` purely for paths, and the
desktop binary resolver would still be separate.

**One binary resolver with the discovery fallback.** The resolver precedence is `$<override>` (if it
names an existing file) → `bin/<name>` or `target/{release,debug}/<name>` under cwd/ancestors →
`~/.local/bin/<name>` → none. This is the desktop host's resolver, generalized and shared. _Why:_ the
missing-fallback drift was the concrete failure; one resolver removes the class of bug.

**Env-var names are constants in this crate.** `ENV_TILLERD_DIR`, `ENV_DAEMON_BIN`, `ENV_GATE_BIN`,
`ENV_NOTIFY_BIN` (and the names the resolvers read) live here; callers reference them. The auth-token
names are out of scope.

**Migrate incrementally, delete as you go.** Each consumer is switched in its own step with its tests
kept green, then its local resolver/builder is deleted in the same step — no parallel duplicate left
behind.

## Risks / Trade-offs

- **A new workspace crate** → more members, longer graph. Mitigation: it is tiny, leaf-level, and
  collapses four resolvers into one; the net is less code.
- **Behavior shift for non-desktop consumers** → they gain the `target/{release,debug}` binary
  fallback they did not have. Intended, but it changes what a bare `cargo run` of a service resolves.
  Mitigation: the override env still wins first; tests assert precedence.
- **Wide blast radius** (7 crates touched) → a migration bug could break boot. Mitigation: per-crate
  steps, workspace `cargo test` + `clippy -D warnings` gate after each, and the existing
  service/boot integration tests.

## Migration Plan

1. Add the `tillerd-paths` crate with the resolvers, builders, binary resolution, and env-name
   constants, fully unit-tested for precedence.
2. Migrate consumers one at a time, deleting each local impl in the same step: `service-host` →
   `process-launch` → `daemon-pty` → `gate` → `mcp-gateway` → `orchestrator` → `desktop`.
3. After each step, run workspace `cargo test` + `cargo clippy --all-targets -- -D warnings`.
4. Rollback is per-step (revert the one consumer); the crate itself is additive until the first
   migration.

## Open Questions

- **Crate directory name:** `crates/paths` (package `tillerd-paths`) vs `crates/tillerd-paths`. Lean
  `crates/paths` to match the existing `crates/<short>` convention (`contracts`, `redact`). The adr
  step can fix this.
- **Auth-token env names:** include `TILLERD_GATE_ADMIN_TOKEN` / `TILLERD_SESSION_ID`/`TOKEN` names as
  constants now, or defer with the values? Deferred here (out of scope); revisit if a second reader
  appears.
- **No in-force ADR is revisited.** The new ADR records `tillerd-paths` as the source of truth and the
  supersession of the scattered resolvers; it changes no path, file name, or env semantics.
