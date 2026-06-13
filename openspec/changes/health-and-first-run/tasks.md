## 1. Service health source (orchestrator + host)

- [x] 1.1 [service-health] Add a runtime-agnostic `read_service_health(specs, probes)` in the orchestrator crate that reads each service's manifest (ADR-0028 discovery source) and derives name, version, and rich state — absent manifest or dead pid -> unavailable; version != expected -> version-mismatch; else the manifest's starting / ready / draining. Live in any boot state (the boot snapshot only covers all-ready); no health probe/socket added to a service. Tests (orchestrator unit, temp manifests + fake probes): "Each supervised service is reported", "Each state is distinguishable", "Version mismatch and draining are distinct from ready", "No mutating operation is exposed".
- [x] 1.2 [service-health] Add the read-only desktop host command returning the per-service status snapshot (new command, NOT an extension of `orchestrator://status` — 0.0.6 freeze); register in `lib.rs` invoke_handler and pin in `command_contract.rs`. Tests: "Status comes from discovery, not a probe"; command_contract arg-shape.

## 2. SDK type and host-agnostic source port

- [x] 2.1 [service-health] Add the `ServiceHealth` type + client method in `packages/sdk/src/orchestrator`, additive alongside the existing orchestrator status types. Tests: status→type mapping (bun:test).
- [x] 2.2 [service-health] Add the `ServiceHealthSource` port + desktop adapter + injectable resolver (returns `null` off the desktop host), mirroring `LogSource` in `apps/ui/app/lib/transport`. Tests: "Desktop host provides the source", "Source absent on an unsupported host".

## 3. Read-only health indicator (UI)

- [x] 3.1 [ui-health-indicators] Aggregate indicator component — a single read-only indicator whose state derives as the worst across the orchestrator boot state and each service (healthy / starting / failed), consuming the source. Tests (bun:test + happy-dom, non-layout): "Aggregate reflects the worst service state", "Starting state while services come up", "No lifecycle controls present".
- [x] 3.2 [ui-health-indicators] Add a shadcn-style `Popover` (the project's base-ui stack, `@base-ui/react/popover` — same family as the existing tooltip, no new dependency); on click the indicator opens a dismissible non-modal panel with one row per service (orchestrator, gate, daemon) showing version, state, failure reason, and a logs link that opens the viewer pre-filtered to that service (reuse the 0.0.7 facet filter); version-mismatch/draining shown inline. Tests: "Panel lists each service", "Panel is dismissible and non-blocking", "Row reveals version and state", "Row links to that service's logs", "Version mismatch shown inline" (content unit; open/dismiss + layout → e2e).
- [x] 3.3 [ui-health-indicators] Wire the aggregate indicator into `AppShell`'s bottom-right cluster (alongside/within the existing `HostStatusBadge`).

## 4. Progressive boot (UI)

- [x] 4.1 [ui-progressive-boot] Extend `useDesktopHost` to carry the per-service health snapshot, resolved lazily and independent of the boot gate, so the shell renders immediately and a failure sets the indicator state without unmounting the shell. Tests: "Shell visible during boot", "Failure shows indicator, shell stays usable".
- [x] 4.2 [ui-progressive-boot] Delayed skeleton (~200ms grace tunable) for daemon/log-dependent content (terminal panes + log viewer) using the existing `ui/skeleton`: render content directly if it resolves within the grace, show the skeleton only after, reveal content when ready; store-backed content (sidebar) renders without a skeleton; show already-available content immediately. Tests: "No skeleton flash on fast resolve", "Skeleton after the grace delay while slow", "Content replaces skeleton when ready", "Available content is shown immediately", "Content backed by an already-open source does not show a skeleton" (timing/branch unit; skeleton render → desktop-e2e).
- [x] 4.3 [ui-progressive-boot] Refresh the snapshot on the existing `orchestrator://status` event (no new event — design decision).

## 5. Verify (fix-all gate)

- [x] 5.1 Extend a desktop-e2e smoke (`tests/desktop-e2e/`) to assert indicators render, skeleton-while-slow, and popover detail in the real webview.
- [x] 5.2 Run `bun run verify` (format:check + check-types + lint + test + e2e) and fix to green; confirm every spec scenario maps 1:1 to a test (`/opsx:verify`).
