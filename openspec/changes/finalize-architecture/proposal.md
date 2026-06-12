# finalize-architecture (0.0.6)

## Why

0.0.6 is the last architecture-changing version of 0.x: the service contract, wire
protocol, data model (ADR-0023), runtime layout (ADR-0025), extension seams, and design
tokens freeze here, and every later 0.x version is additive on these seams. The freeze
only holds if the seams are complete (lifecycle, discovery, upgrade story, log
correlation) and verifiable (a desktop E2E suite instead of manual checks).

## What Changes

- Desktop E2E suite — extend the existing rig (`tests/desktop-e2e/run.sh`, WebdriverIO +
  `tauri-webdriver` over test-gated `tauri-plugin-webdriver`) from three smoke specs to a
  solid suite: boot to ready in dev and bundled modes, full project/session create flows,
  resume after restart, runs in CI. Lands first so every other 0.0.6 item verifies
  against it.
- Dynamic-ACL contract test — runtime arg-shape verification for every desktop IPC
  command (deferred from the GUI arg-shape work): tauri `dynamic-acl` feature +
  `add_capability` before building the webview, then `get_ipc_response` per command
  asserting the error is never not-found/deserialize.
- `service-host` solidified — the `Service` trait gains first-class ready/drain
  lifecycle phases and the discovery convention (socket/manifest); health (ADR-0019) and
  identity/version are already in. Gate + daemon conform; future services inherit the
  contract. Health feeds the 0.0.8 indicators.
- **BREAKING** Daemon upgrade becomes drain-and-restart — on version mismatch the daemon
  drains (refuses new sessions, lets active ones finish), the supervisor swaps the
  binary and starts fresh. Built on the contract's drain primitive. Supersedes
  fd-handoff (ADR-0011) and dissolves the in-process-fd mandate (ADR-0010); absorbs the
  deferred `daemon-upgrade-drain-restart` change (re-authored here against the Rust
  orchestrator per its revival condition).
- `correlation_id` threaded across hops in structured logs — the log-viewer (0.0.7),
  health surfacing (0.0.8), and every later feature join records on it.
- Design tokens finalized — apply `apps/ui/DESIGN.md` across the existing shell and
  close its token-level gaps: motion/transition scale, icon sizing token, light-mode
  counterparts.
- Dead-code sweep — delete the retired TS packages left from the Rust inversion
  (`engine`, `platform-bun`, `adapter-claude-code`, TS `daemon-pty`/`gate-client`, ...)
  where nothing live references them; dormant `apps/server` keeps only what it needs
  until its 0.1.4 rewrite.

## Capabilities

### New Capabilities

(none — all changes land on existing capabilities)

### Modified Capabilities

- `service-contract`: ready/drain lifecycle phases and the socket/manifest discovery
  convention become requirements of the `Service` trait.
- `daemon-upgrade`: the fd-handoff requirement is replaced by drain-and-restart
  (**BREAKING** — planned upgrades no longer preserve live sessions; resume covers them).
- `orchestrator-supervision`: version mismatch triggers drain, binary swap, fresh start
  instead of handoff orchestration.
- `observability-logging`: `correlation_id` propagates across every service hop and
  appears in all structured log records.
- `dev-verification`: the desktop E2E suite and the dynamic-ACL contract test become
  required gates.
- `ui-shell`: rendering is bound to the finalized DESIGN.md token set, including motion
  scale, icon sizing token, and light-mode tokens.

## Impact

- `crates/service-host` — lifecycle phases, discovery convention.
- `crates/orchestrator` — supervision drain/swap/restart path, correlation_id plumbing.
- Daemon + gate services — conform to the extended contract; drain state machine.
- `apps/desktop/src-tauri` — dynamic-ACL contract test (`command_contract.rs`).
- `apps/ui` — token application + DESIGN.md gap closure.
- `tests/desktop-e2e` — suite extension, CI wiring.
- Retired TS packages under `packages/` — deleted; `apps/server` trimmed.
- ADRs: new ADR supersedes ADR-0011 and lifts ADR-0010's in-process-fd constraint.
- Closes the deferred `daemon-upgrade-drain-restart` change.
