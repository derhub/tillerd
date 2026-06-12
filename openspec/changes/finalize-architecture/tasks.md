# finalize-architecture — tasks

## 1. E2E foundation (lands first; everything later verifies against it)

- [x] 1.1 Extend `tests/desktop-e2e` to the full suite: boot-to-ready in dev and bundled
  modes, full project/session create flows, resume after restart; wire into CI
  (dev-verification spec, design D6). [live run deferred to user]
- [x] 1.2 Runtime arg-shape contract test in `apps/desktop/src-tauri/src/command_contract.rs`:
  every IPC command invoked via `get_ipc_response` over the real context + local origin,
  asserting never not-found/deserialize (dev-verification spec, design D7).

## 2. Service contract and upgrade

- [~] 2.1 `service-host` lifecycle: ready handle on `ServeContext`, SIGUSR2 drain phase,
  manifest `status` + `socket_path` discovery; gate + daemon conform; E2E/orchestrator
  read readiness from the manifest (service-contract spec, ADR-0028).
  [done: contract (ready/drain/manifest fields) + daemon/gate conform, tested. deferred:
  orchestrator consuming manifest `status` for readiness — entangled with deduping the
  3+ ManifestData/adopt-or-spawn impls; do with that refactor.]
- [~] 2.2 Drain-and-restart: daemon drain state machine (refuse-new, wait-for-idle,
  explicit upgrade-now), orchestrator supervision drains/swaps/restarts on version
  mismatch, resume-after-restart via workspace persistence (daemon-upgrade +
  orchestrator-supervision specs, ADR-0029).
  [done: daemon drain SM (refuse-new EDRAINING + idle-exit) + orchestrator drain-on-
  mismatch + resume (resume_all + resume.smoke.ts), tested. upgrade-now = SIGTERM.
  deferred: removing the fd-handoff/snapshot machinery (REMOVED reqs) → 5.1 sweep.]

## 3. Observability

- [ ] 3.1 `correlation_id`: generate at ingress (desktop IPC / surface op), bind into
  logger context, carry on request envelopes orchestrator -> daemon/gate, add the key to
  the standardized vocabulary (observability-logging spec, design D5).

## 4. Design tokens

- [ ] 4.1 Close DESIGN.md token gaps (motion/transition scale, icon sizing token,
  light-mode counterparts), then apply tokens across the shell with no ad-hoc values
  outside the terminal palette exemption (ui-shell spec, design D8).

## 5. Sweep and gate

- [ ] 5.1 Dead-code sweep: delete retired TS packages (`engine`, `platform-bun`,
  `adapter-claude-code`, TS `daemon-pty`/`gate-client`, ...) where check-deps + workspace
  references show nothing live; trim dormant `apps/server`; close the deferred
  `daemon-upgrade-drain-restart` change (absorbed here).
- [ ] 5.2 Final gate: run `/opsx:verify` and fix all issues, then `bun run verify` and
  fix all issues, then `bun run e2e` and fix all issues.
