## Context

The renderer already has a working keybinding layer — `commands/keybindings.ts` (canonical `Accelerator` format, `Chord`, parse/format/canonicalize, mac display, `eventToAccelerator`, four presets, user overrides), `useKeybindings.ts` (resolve preset+overrides → `Map<id,accel>`, global keydown dispatch with capture-target guards, backend leader key), and `CommandCenter.tsx` (cmdk palette reading `useCommands()` + resolved bindings). What is missing is a single declarative source of truth. A command's identity is in `ids.ts` (`ACTION`, `ACTION_TITLES`), its default key is in `keybindings.ts` (`PRESETS`), and its handler is registered at runtime from `RootLayout`, `useShellCommands`, and `PanelContent` via `<RegisterCommands>`. There is no context gating (the palette lists all commands unconditionally), no surface/menu model, no icons, and no toggle state.

The chosen scope (from interview): minimal `when` context keys, minimal surface tags, first-class toggle commands, single-chord only. This change is the foundation the desktop title bar (PR #64, paused) consumes.

## Goals / Non-Goals

**Goals:**

- One `CommandDef` table: identity + metadata + default keys + surfaces + `when` + toggle, declared in one place.
- Handlers registered by id, decoupled from declaration, closing over live context.
- A minimal context-key store + `when` evaluator gating palette visibility, keybinding activation, and enablement.
- Surface tags (`palette` / `titlebar` / `contextmenu`) + `group` so the palette and the title bar toolbar are data-driven.
- First-class toggle commands with a reactive checked selector.
- Preserve the `Accelerator` format, presets, overrides, leader key, settings keys, and the `useCommands()`/palette contracts.

**Non-Goals:**

- Multi-key sequences / leader chords (single-chord only; deferred).
- A full VSCode when-clause expression grammar (`||`, `==`, `=~`, parens) — minimal AND/negation only.
- A full menus contribution model with per-item when-clauses and ordering groups — surface tags only.
- Backend/orchestrator command-library changes (`command-library` is unrelated).
- Rebindable-UI redesign — the existing keybinding settings panel keeps working against the new source.

## Decisions

### D1: Command contribution model — declaration split from handler

A command is a static `CommandDef`:

```
interface CommandDef {
  id: ActionId;
  title: string;
  category?: string;
  keywords?: string[];
  icon?: LucideIcon;              // for toolbar/menu surfaces
  surfaces?: Surface[];          // where it appears; default ["palette"]
  group?: string;                // ordering/section within a surface
  defaultKeys?: Partial<Record<PresetName, Accelerator>>;
  when?: WhenExpr;               // context gate; absent = always
  toggle?: (ctx) => boolean;     // present => toggle command; returns checked
}
```

Definitions live in one table (`commands/defs.ts`), replacing `ACTION_TITLES` and `PRESETS` as the source of truth (the `ACTION` id constants stay — they are the stable keys). Handlers register separately by id: `registerCommand(id, handler)` (or a `useCommand(id, handler)` hook), so a handler can close over `navigate`, store setters, and session context while the declaration stays static and testable. The merged runtime `Command` = `def` + resolved `handler` + resolved `accelerator` + resolved `checked`. Alternative rejected: keep `run` in the def — impossible, handlers need live React context.

### D2: Minimal `when` context system

A `ContextStore` holds named boolean/string keys pushed in by features (`setContextKey`, VSCode's `setContext` model). `WhenExpr` is a minimal form: a list of terms ANDed together, each term a key name or a negated key (`!terminalFocus`). Evaluator is ~20 lines, no parser dependency. `when` gates three things: palette visibility (hidden when false), keybinding activation (dispatch skips a binding whose command's `when` is false), and command enablement. Alternative rejected: full grammar — more code and test surface than the scope needs; the format is forward-compatible (a richer expr type can replace the term-list later without changing call sites).

**Availability vs `when`.** A command is *active* only while a handler is registered for its id (handlers register when their contributor mounts). Active-ness is the primary gate: an inactive command is absent from every surface and its keybinding does not fire. `when` is a *further* filter on top, for commands that must be conditionally hidden even while their contributor is mounted. The panel/surface commands (`surfaceSpawn`, `panelSplit*`, etc.) are correctly scoped by handler presence alone — their handlers register only while the panel host (`PanelContent`) is mounted — so they carry no `when`. As shipped, no command needs a `when`; the machinery and its context store land for the title bar's toggle commands (PR #64) and are covered by unit tests. Context keys are seeded on demand by whichever feature consumes them, not eagerly.

### D3: Surface tags drive the palette and the title bar

`Surface = "palette" | "titlebar" | "contextmenu"`. The palette renders active commands whose `surfaces` includes `palette` (default) and whose `when` passes. (Category grouping is declared on `CommandDef` but the palette renders flat until a command sets a `category`.) The title bar toolbar renders commands tagged `titlebar` in `group` order, as icon buttons; a toggle renders active/inactive from its `checked`. This makes the title bar toolbar a pure projection of the command table — no hand-wired buttons. Alternative rejected: full menus map — unnecessary indirection for three surfaces.

### D4: First-class toggle commands

A def with a `toggle` selector is a toggle command. The runtime resolves `checked = toggle(ctx)` reactively (via the context/store subscriptions the selector reads). The palette shows a check mark; title bar buttons render pressed/unpressed and set `aria-pressed`. Invoking the command runs its handler (which flips the underlying state); `checked` updates from the source of truth, never stored twice. The title bar's left/right/bottom panel toggles and the command-palette toggle become toggle defs. Alternative rejected: plain commands + buttons reading the store directly — works but scatters toggle logic and loses palette check rendering.

### D5: Keybindings sourced from the table; format preserved

`PRESETS` is derived from `defs` (`def.defaultKeys[preset]`), so a command's default key lives beside its declaration. `resolveBindings(preset, overrides)`, the `Accelerator`/`Chord` format, `displayAccelerator`, overrides, preset selection, and leader key are unchanged — only their input source moves. `useGlobalShortcuts` additionally checks `when` before firing. Settings keys (`KEYBINDINGS_PRESET_KEY`, `KEYBINDINGS_OVERRIDES_KEY`, `KEYBINDINGS_LEADER_KEY`) are untouched, so existing user config survives. Single-chord only; the format already rejects sequences.

### D6: Migration preserves the `useCommands()` contract

The registry keeps exposing a merged `Command[]` via `useCommands()` (now `def`+`handler`+`accel`+`checked`), so `CommandCenter` and `useGlobalShortcuts` change internally but their consumer shape is stable. Existing registration sites migrate from building `Command[]` inline to `useCommand(id, handler)` calls; each migration is behavior-preserving and independently testable. The `command-center` spec's already-stated "available in context" requirement becomes real.

## Risks / Trade-offs

- **Broad migration across every registration site** → migrate incrementally; the `useCommands()` output contract stays fixed so the palette/dispatch keep working after each step; cover with tests per site.
- **`when` gating silently hides a command** → default `when` absent = always available; add context keys conservatively; test each gated command in both states.
- **Toggle `checked` desync from source of truth** → `checked` is always computed from the store via the selector, never persisted; the handler mutates only the store.
- **Reactivity of `when`/`checked`** → context keys live in a `@tanstack/react-store` slice so the palette and toolbar re-render on change, consistent with `uiStore`/`settingsStore` patterns.
- **Format churn breaking user overrides** → `Accelerator` format and settings keys are frozen; only the default-key source moves.

## Migration Plan

Additive then subtractive: introduce `defs.ts` + context store + `when` evaluator + `registerCommand`, wire the palette/dispatch to read them, migrate registration sites one group at a time, then delete `ACTION_TITLES`/`PRESETS` once nothing reads them. No persisted-data migration (settings keys stable). Rollback = revert; user keybinding config is untouched.

## Open Questions

- Exact initial context-key set beyond `terminalFocus` / `isDesktopHost` / `hasActiveSession` / `commandPaletteOpen` — added as consumers need them; not blocking.
- Whether the keybinding settings panel should surface `when`/surface metadata — out of scope; it keeps editing bindings by id.
