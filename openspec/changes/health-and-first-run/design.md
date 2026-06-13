## Context

The orchestrator boots through `Status` transitions (booting / opening-store /
supervising / ready / failed) emitted over the existing `orchestrator://status`
event and read by a snapshot command. The UI consumes only this coarse signal:
`useDesktopHost` maps it to `{web|booting|ready|error}` and `AppShell` renders a
single bottom-right `HostStatusBadge`.

Per-service detail already exists but stops at the orchestrator: `boot.rs` holds
`services: Vec<ServiceStatus { name, version, liveness, pid, adopted }>` exposed
via `service_statuses()`, derived from each service's manifest in `supervision.rs`
(manifest is the single discovery source, ADR-0028; version mismatch triggers
drain-restart, ADR-0029). None of it reaches the renderer, and a boot failure
collapses the shell to a bare red badge.

The 0.0.7 log viewer established the host-agnostic pattern to reuse: a `LogSource`
port (`apps/ui/app/lib/transport/log-source.ts`) with a desktop adapter and an
injectable resolver that returns `null` off the desktop host. The architecture is
frozen at 0.0.6 — every 0.x change is additive on its seams.

## Goals / Non-Goals

**Goals:**

- Surface per-service status (gate, daemon) to the desktop UI, read-only.
- Make boot progressive: shell first, skeletons for slow service-dependent
  content, failure degrades to an indicator — never a blocking wall.
- Stay additive on frozen seams; reuse the orchestrator-status and log-source
  patterns rather than inventing new ones.
- Keep the source host-agnostic so a server adapter lands later without touching
  indicator or boot logic.

**Non-Goals:**

- No retry/restart controls (supervision/drain seam is frozen).
- No health probe, socket, or route on any service.
- No host-runtime-prerequisite checks; no agent-CLI version awareness (1.0.0).
- No server/web adapter implementation (port shape only).
- No setup wizard or settings (0.0.9).

## Decisions

### New command, not an extension of `orchestrator://status`

A new read-only snapshot command returns the per-service status list; the existing
`orchestrator://status` message is left unchanged. Alternative — adding a
`services` field to the existing status payload — was rejected: it mutates a wire
message frozen at 0.0.6 and couples per-service detail to the boot lifecycle. A
separate command is cleanly additive and independently testable. The new command
is registered in `lib.rs` and pinned by the `command_contract.rs` arg-shape test.

### Status is read live from each manifest, not from the boot snapshot

`boot()` is all-or-nothing: it fails fast and returns `Err` (→ `Status::Failed`,
no `Orchestrator`) if any service is unavailable, so `service_statuses()` is
populated only on full success — it cannot report which service is down or any
non-ready state. The health source therefore reads each service's manifest
directly (the ADR-0028 single discovery source) at query time, via a new
runtime-agnostic `orchestrator` function (`read_service_health(specs, probes)`).
This is live in every state — boot success or failure — and reuses the manifest
the supervisor already trusts. No health endpoint is opened on gate or daemon
(honors `service-host/host.rs`: health "never serialized over a wire — no health
socket or route"). Putting the function in the orchestrator crate (not the desktop
host) keeps it host-agnostic: the desktop command and a future server handler call
the same function.

Each service carries a rich state — starting / ready / draining / version-mismatch
/ unavailable — derived from the manifest lifecycle status, pid liveness, and the
expected-version comparison: absent manifest or dead pid -> unavailable; version
!= expected -> version-mismatch; otherwise the manifest's starting / ready /
draining. A flat up/down was rejected: it cannot honor the spec's requirement that
version-mismatch and draining read distinctly from ready. The orchestrator-level
boot `Status` (already surfaced via `orchestrator://status`) supplies the
orchestrator's own row in the panel; the UI composes the two.

### Live updates ride the existing status event; no new event in 0.0.8

Service status is low-churn (it changes at boot and on drain-restart). The UI
loads a snapshot via the new command and re-queries when the existing
`orchestrator://status` event fires. Adding a dedicated per-service event was
considered and deferred — it adds a wire surface for no current benefit, and the
existing event already brackets every transition that can change service status.

### Host-agnostic source port mirrors `LogSource`

A `ServiceHealthSource` port (one method: snapshot the per-service status) with a
desktop adapter and an injectable resolver that returns `null` off the desktop
host — the same shape and test seam as `LogSource`. The SDK carries the
`ServiceHealth` type (name, version, liveness, version-match/draining state)
alongside the existing orchestrator status types. Consumers depend on the port,
never the adapter, so a server adapter is a drop-in later.

### Progressive boot via the existing host-state hook

`useDesktopHost` already renders the shell during `booting`. Extend its state to
carry the per-service health snapshot (resolved lazily, independent of the
boot gate) so `AppShell` renders immediately and service-dependent panes show a
skeleton until their data resolves. A boot failure sets the aggregate indicator
state; the shell stays mounted. No new top-level loading screen is introduced.

Skeleton scope is daemon/log-dependent content only — terminal panes and the log
viewer, which wait on the daemon and on log files. The session sidebar reads from
the store, which the orchestrator opens early in boot (before `ready`), so it
renders from store data without a skeleton. Alternative — skeletoning every
service-dependent region including the sidebar — was rejected: it adds skeletons
the user would notice on a cold boot for data that is in fact available early.

### Delayed skeleton to avoid flash (~200ms grace)

Service-dependent content shows the existing `ui/skeleton` placeholder only after
a ~200ms grace delay; content that resolves faster renders directly with no
skeleton. This keeps a fast boot visually quiet (the user-stated goal). The delay
is a single tunable; ~200ms is the chosen default (long enough to skip the flash
on a warm boot, short enough to feel responsive on a cold one). Alternative —
skeleton immediately on pending — was rejected because a sub-100ms flash is itself
noticeable.

### Single aggregate indicator with a Popover panel

One read-only aggregate indicator lives in the bottom-right cluster; its state is
the worst across the orchestrator boot state and each service. Clicking it opens a
shadcn `Popover` panel listing one row per service (orchestrator, gate, daemon)
with version, state, failure reason, and a logs link that opens the logs viewer
pre-filtered to that service (the viewer already supports per-service facet
filtering, 0.0.7). `Popover` is chosen over alternatives: it is click-triggered
(works for mouse and touch), non-modal (dismiss on outside-click/Esc, never dims
or blocks the shell — fits the invisible-boot goal), and holds arbitrary content.
`Tooltip`/`HoverCard` are hover-only and meant for small labels; `Dialog`/`Sheet`
are modal/drawer and too heavy; `DropdownMenu` is for actions, not a status panel.
It is the same Radix family already vendored for `Tooltip`, so the only new
dependency is `@radix-ui/react-popover` (installed via the shadcn CLI), consistent
with the existing UI stack. No lifecycle controls appear in the panel.

## Risks / Trade-offs

- [Snapshot goes stale between status transitions] → re-query on every
  `orchestrator://status` event; service status only changes at boot/drain, which
  the event already brackets. A dedicated event remains an additive future option.
- [Skeleton, popover, and slow-load behavior are layout-dependent] → unit tests
  (bun:test + happy-dom) cover status→indicator mapping, source resolution, and
  loading-vs-content branch logic only; skeleton-while-slow and popover render go
  to desktop-e2e (happy-dom has no layout — testing memory).
- [Pressure to extend the frozen status payload] → the `command_contract.rs`
  arg-shape test and a new-command-only rule keep the change additive; the design
  forbids touching the existing message.
- [Source absent off the desktop host] → the resolver returns `null` and the
  indicator/skeleton consumers degrade gracefully, exactly as the log viewer does.

## Migration Plan

Additive only — no data model or schema change, no migration. The feature is
chrome plus a read-only command; rollback is a straight revert of the change with
no persistent state to unwind.

## Open Questions

None. All planning decisions (scope, version-mismatch handling, failure depth,
onboarding shape) were resolved before proposal.
