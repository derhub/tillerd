## Why

The desktop surfaces only one coarse signal today: an orchestrator-level boot
status (booting / opening-store / supervising / ready / failed) shown as a single
bottom-right badge. The orchestrator already collects per-service status for the
gate and daemon (name, version, liveness, version match), but that detail never
reaches the user, and a boot failure degrades to a bare red badge with no
graceful, progressive reveal. Roadmap 0.0.8 (Health and first-run UX) closes this:
make per-service health visible and make boot feel instant and unobtrusive.

## What Changes

- Surface per-service health (gate, daemon) to the desktop UI through a new,
  **additive** host command + event and a new SDK type — read from the
  orchestrator's existing manifest-derived view. No health socket or route is
  added to any service (the manifest stays the single discovery source).
- Render health as read-only chrome: a single aggregate indicator in the existing
  bottom-right cluster (its state is the worst across services) that, on click,
  opens a dismissible non-modal panel listing every service (orchestrator, gate,
  daemon) with version, liveness, the failure reason, and a link to the logs
  viewer. A service running the wrong version or draining surfaces here (the
  roadmap's "version out of range"), not as a separate screen.
- Make boot progressive: the app shell renders immediately, service-dependent
  content lazy-loads and shows a skeleton placeholder while a service is starting
  or slow, and failures degrade to the subtle indicator rather than a blocking
  modal. No setup wizard, no first-run wall.
- The health source is a host-agnostic port with a desktop adapter now; a future
  server/web adapter satisfies the same shape without touching indicator or boot
  logic (mirrors the 0.0.7 log-source split).

Non-goals: no retry/restart actions (read-only — the supervision/drain seam is
frozen); no host-runtime-prerequisite checks; no agent-CLI version awareness
(deferred to 1.0.0); no settings/secrets (0.0.9); no server adapter implementation
(port shape only).

## Capabilities

### New Capabilities

- `service-health`: the orchestrator exposes per-service status (name, version,
  liveness, version-match state) to the host through an additive command and
  event; a host-agnostic source port with a desktop adapter delivers it to the
  UI, and the SDK carries the type. Manifest-derived, read-only, no new service
  wire.
- `ui-health-indicators`: read-only per-service health chrome in the app shell —
  a status dot per service with a popover (version, liveness, failure reason,
  logs link); version-mismatch and draining states are surfaced here.
- `ui-progressive-boot`: the shell renders before services are ready;
  service-dependent content lazy-loads with a skeleton placeholder while a
  service is starting or slow, and a service failure degrades to the health
  indicator rather than a blocking screen.

### Modified Capabilities

None. The change is additive on the seams frozen at 0.0.6: it introduces a new
command, event, and SDK type rather than altering the existing
`orchestrator://status` message, and it reads service status from the existing
manifest discovery path without changing `service-contract`,
`orchestrator-supervision`, or `sdk-orchestrator-client` requirements.

## Impact

- `crates/orchestrator` — a new runtime-agnostic function that reads each
  service's manifest (the ADR-0028 discovery source) and derives its rich state,
  live in any boot state (additive; the boot snapshot only covers the all-ready
  case).
- `apps/desktop/src-tauri` — new read-only command returning per-service status
  and a status event; registered in `lib.rs` and the `command_contract.rs`
  arg-shape test.
- `packages/sdk/src/orchestrator` — new per-service health type and client
  method, additive alongside the existing orchestrator status client.
- `apps/ui/app` — a host-agnostic health source port + desktop adapter
  (`lib/transport`), a health-indicator component and popover, app-shell wiring,
  and skeleton/lazy-load handling for service-dependent content.
- No new crates or packages; no new dependencies. Honors the 0.0.6 architecture
  freeze (additive only) and the host-agnostic-by-design rule.
