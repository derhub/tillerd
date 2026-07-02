# Tasks — freeze-docs-specs

## 1. ADR statuses

- [x] 1.1 0041/0042/0043 -> accepted; 0035 -> superseded (by 0036); 0037 back-annotate the 0041
  clause revision (0039 pattern).

## 2. Spec truth

- [x] 2.1 `generated-entity-hooks` delta: REMOVED emitter requirement, ADDED runtime-factory
  requirement matching the shipped `query()`/`command()`/`subscribe()` design.
- [x] 2.2 `ui-terminal-pane`: host-mapped transport wording (desktop `channel` verb, server
  WebSocket).

## 3. Doc truth

- [x] 3.1 `docs/tanstack-client-engine.md`: cross-window section describes the shipped broadcast
  design (Tauri event bus, focus-refetch disabled, coalesce guard, BroadcastChannel unfit).

## 4. Purposes

- [x] 4.1 Replace the 29 `TBD` purpose lines with real purposes derived from each spec's
  requirements (terse, current-state, no roadmap speak).

## 5. Gate

- [ ] 5.1 `openspec validate` (or structural check) passes; `bun run verify` unaffected (no code);
  memory index/topic fixes proposed to the user separately (memory protocol: show, ask, then
  write).
