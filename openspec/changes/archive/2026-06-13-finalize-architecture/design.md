# finalize-architecture — design

## Context

0.0.6 freezes the architecture for the rest of 0.x (service contract, wire protocol,
data model, runtime layout, design tokens). Current state:

- `service-host` (`crates/service-host`) runs services via the `Service` trait: config,
  serve, shutdown, health (ADR-0019). Manifest is written before serve; health knows
  `Serving`/`Draining` but there is no first-class ready signal, no drain phase distinct
  from shutdown, and no documented discovery convention.
- Daemon upgrade is still specced as fd-handoff (ADR-0011, `daemon-upgrade` spec), but
  the machinery lived in the retired TS engine; the Rust daemon has no handoff. The
  deferred `daemon-upgrade-drain-restart` change's revival condition is met.
- The desktop E2E rig exists (`tests/desktop-e2e/run.sh`, WebdriverIO +
  `tauri-webdriver`; boot, project/session, terminal smoke specs) but is not a suite and
  not in CI.
- Loggers bind context (ADR-0012) but no `correlation_id` crosses process hops.
- `apps/ui/DESIGN.md` documents tokens with known gaps: motion/transition scale, icon
  sizing token, light-mode counterparts; the shell predates full token application.
- Retired TS packages (`engine`, `platform-bun`, `adapter-claude-code`, TS
  `daemon-pty`/`gate-client`, ...) remain in `packages/`.

## Goals / Non-Goals

**Goals:**

- Every seam frozen at 0.0.6 is complete and exercised by an automated gate.
- E2E suite lands first; every other item verifies against it.
- Drain primitive lives in the service contract once; daemon upgrade and future services
  reuse it.

**Non-Goals:**

- Subprocess-per-session (drain-and-restart only removes the constraint forbidding it).
- Crash recovery for unplanned crashes (unchanged; ADR-0008 posture holds).
- Log-viewer (0.0.7), health indicators (0.0.8) — they consume these seams later.
- Windows support; agent surface (1.x).

## Decisions

### D1. Drain is a contract-level signal, not a wire frame

Drain is triggered by a host-level signal (SIGUSR2) handled by `service-host`, mirroring
how stop signals already work (`signals.rs`). On drain the host flips the service into a
`drain()` trait phase: refuse new work, finish active work, report `Draining` health,
update the manifest status, exit when idle.

- Alternative: a drain frame on each service's wire protocol. Rejected: couples every
  protocol to lifecycle (the exact coupling fd-handoff created), and the wire protocol
  freezes at 0.0.6 — lifecycle must not be a reason to touch it later.
- SIGTERM keeps its current meaning (graceful shutdown now); SIGUSR2 means drain
  (graceful refusal, user-paced exit).

### D2. Ready is a ServeContext handle plus manifest status

`ServeContext` gains a `ready` handle the service calls once it is listening. The host
records the transition (log + manifest status `starting -> ready`). The manifest gains
`status` (`starting | ready | draining`) and `socket_path` fields; the orchestrator's
adopt-or-spawn and the E2E rig read readiness from the manifest instead of poll-connect
guessing.

- Alternative: infer readiness from socket connect. Rejected: races spawn-vs-listen and
  cannot distinguish starting from wedged.

### D3. Discovery convention: the manifest is the source of truth

A service is discovered by reading its manifest under the runtime layout (ADR-0025):
name, version, pid, status, socket path. No port files, no scanning. Clients resolve
sockets via manifest only.

### D4. Daemon upgrade: drain-and-restart, supervisor-driven

On version mismatch (manifest version vs expected), the orchestrator's supervision sends
drain, waits for idle (no auto-kill timer; an explicit "upgrade now" path terminates
active sessions deliberately), swaps the binary, starts fresh. Resume after restart goes
through existing workspace persistence + the agent CLI's own resume. Supersedes ADR-0011
(new ADR records this); ADR-0010's in-process-fd mandate is no longer forced.

- Alternative: keep fd-handoff. Rejected per the absorbed proposal: heaviest daemon
  subsystem serving a property a single local user rarely needs.

### D5. correlation_id rides existing context binding

A `correlation_id` (UUID) is generated at the operation ingress (desktop IPC command or
surface operation), bound into the logger context (ADR-0012 binding; tracing span field
on the Rust side), and passed on cross-process calls as a field on the existing request
envelopes (orchestrator -> daemon, orchestrator -> gate). Standardized key:
`correlation_id` in the shared attribute vocabulary. No tracing infrastructure, no
spans-over-the-wire — one string field.

### D6. E2E suite is the gate bed and lands first

Extend `tests/desktop-e2e` to: boot-to-ready in dev and bundled modes, full
project/session create flows, resume after restart; wire into CI. Suite ordering is a
hard sequencing constraint: later 0.0.6 items (drain-and-restart, tokens) add specs to
it rather than manual checks.

### D7. Dynamic-ACL contract test inside the existing rig

`apps/desktop/src-tauri/src/command_contract.rs` grows the runtime arg-shape check:
tauri `dynamic-acl` feature, `app.handle().add_capability(...)` before the webview, then
`get_ipc_response` per command asserting the error is never not-found/deserialize.
Feature propagation must reach the dev-dependency build (`use tauri::Manager`).

### D8. Tokens close in DESIGN.md first, then apply

Add the missing token definitions to DESIGN.md (motion/transition scale, icon sizing
token, light-mode counterparts), then apply across the shell as CSS variables. Token
names are frozen at 0.0.6; later UI consumes them unchanged.

### D9. Dead-code criterion: nothing live references it

Delete a retired package only when `turbo run check-deps` + workspace references show
nothing live importing it. `apps/server` stays dormant-but-trimmed until 0.1.4.

## Risks / Trade-offs

- [E2E flakiness in CI (WKWebView + webdriver)] -> keep specs deterministic
  (manifest-based readiness from D2 removes the biggest poll race); bundled-mode boot
  spec isolated so a runner-specific failure does not mask the rest.
- [Drain waits forever on a never-idle session] -> no auto-kill by design; the explicit
  "upgrade now" terminate path is the pressure valve and is user-paced.
- [BREAKING: planned upgrades drop live sessions] -> accepted, matches ADR-0008 crash
  posture; resume-after-restart covers continuity.
- [Token application churns many components] -> tokens are defined before application
  (D8); E2E smoke guards behavior, visual regressions reviewed manually.
- [Sweep deletes something referenced dynamically] -> `bun run verify` + desktop E2E
  after the sweep; deletions are a single revertable commit.

## Migration Plan

Pre-v1, no deployments: land on main behind the verify battery. Rollback is git revert.
The deferred `daemon-upgrade-drain-restart` change is closed by this one (its DEFERRED.md
revival condition is satisfied here).

## Open Questions

- None blocking. ADR step records: drain-and-restart superseding ADR-0011 (and lifting
  ADR-0010's mandate), and the lifecycle/discovery extension alongside ADR-0019.
