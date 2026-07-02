# freeze-docs-specs

## Why

The code phases (1-3) aligned the implementation to the target architecture; the written record
lags it. Four ADRs implemented on this branch still say `proposed`, one spec describes an
abandoned build-time emitter, the canonical client-engine doc contradicts the shipped
cross-window design, and 29 specs carry `TBD` purpose lines. A freeze whose documents disagree
with the code will mislead the UI overhaul; this change makes the record match reality.

## What Changes

- ADR statuses: 0041/0042/0043 `proposed` -> `accepted`; 0035 -> superseded by 0036 (which
  already declares it); 0037 back-annotated for 0041's revision of its bus-exclusivity clause
  (same pattern 0039 uses for 0040).
- `generated-entity-hooks` spec rewritten to the shipped design: generic runtime
  `query()`/`command()`/`subscribe()` factories over generated types — the build-time hook
  emitter was abandoned (autogen-sdk change record).
- `docs/tanstack-client-engine.md` cross-window section rewritten to the shipped design: live
  invalidation broadcast over the Tauri event bus, `refetchOnWindowFocus` disabled
  (BroadcastChannel unfit across separate webview processes).
- `ui-terminal-pane` spec wording: host-mapped transport (desktop duplex `channel` verb; server
  host WebSocket), replacing the WebSocket-only description.
- 29 spec `TBD` purpose lines replaced with real one-to-three-sentence purposes derived from
  each spec's requirements.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `generated-entity-hooks`: requirement rewritten from build-time hook emitter to runtime
  factories over generated bindings.
- `client-engine`: the stale generated-hook-surface requirement removed (same abandonment; the
  runtime-factory requirement in `generated-entity-hooks` covers entity access).

## Impact

- `docs/adr/` (5 status lines), `docs/tanstack-client-engine.md` (one section),
  `openspec/specs/` (30 files: 29 purposes + terminal-pane wording), delta for
  `generated-entity-hooks`. No code.
