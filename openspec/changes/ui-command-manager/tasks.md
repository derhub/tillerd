## 1. Context-key store and `when` evaluator

- [ ] 1.1 Add `app/lib/commands/context.ts`: a reactive context-key store (`@tanstack/react-store` slice) with `setContextKey`/`useContextKey` and a `readContext()` snapshot.
- [ ] 1.2 Add `app/lib/commands/when.ts`: `WhenExpr` (conjunction of key terms + negation) + `evaluateWhen(expr, ctx)`; absent expr = always true.
- [ ] 1.3 Seed context keys: `terminalFocus` (from the existing capture-target/xterm-focus guard), `isDesktopHost`, `hasActiveSession`, `commandPaletteOpen`.
- [ ] 1.4 Tests: evaluator truth table (key present/absent, negation, conjunction, empty expr); context store reactivity.

## 2. Command definition model

- [ ] 2.1 Extend the registry types in `app/lib/commands/registry.tsx`: `CommandDef` (id, title, category?, keywords?, icon?, surfaces?, group?, defaultKeys?, when?, toggle?) and runtime `Command` = def + handler + accel + checked.
- [ ] 2.2 Add `Surface = "palette" | "titlebar" | "contextmenu"`; default surfaces `["palette"]`.
- [ ] 2.3 Add `app/lib/commands/defs.ts`: the single definitions table for all existing actions, migrating `ACTION_TITLES` and `PRESETS.default` (+ other presets) into per-def metadata + `defaultKeys`.
- [ ] 2.4 Tests: every `ACTION` id has a def; default-keys parity with the pre-migration `PRESETS`.

## 3. Handler registration by id

- [ ] 3.1 Add `registerCommand(id, handler)` / `useCommand(id, handler)` in the registry; runtime `Command` composes def + handler; missing handler => inert (no-op invoke).
- [ ] 3.2 Update `useCommands()` to return the composed runtime commands (def + handler + resolved accel + resolved checked), preserving the existing consumer shape.
- [ ] 3.3 Tests: registered handler runs on invoke; defined-but-unregistered command lists but no-ops.

## 4. Toggle commands

- [ ] 4.1 Support `toggle` selector on `CommandDef`; resolve `checked = toggle(ctx)` reactively in `useCommands()`.
- [ ] 4.2 Tests: checked follows the underlying state; invoke runs handler; checked never stored separately.

## 5. Keybindings sourced from definitions, gated by `when`

- [ ] 5.1 Derive presets from `defs` (`def.defaultKeys[preset]`) instead of the standalone `PRESETS` map; keep `resolveBindings`, `Accelerator`/`Chord`, `displayAccelerator`, overrides, leader, and settings keys unchanged.
- [ ] 5.2 Gate `useGlobalShortcuts` dispatch on the command's `when` (skip a binding whose `when` is false); keep capture-target guards.
- [ ] 5.3 Tests: default key from def; override wins + persists + clears to default; gated key does not fire out of context; single-chord preserved.

## 6. Palette reads the single registry

- [ ] 6.1 `CommandCenter.tsx`: filter to `surfaces` including `palette` and passing `when`; group by category; render toggle checked state; keep binding display, fuzzy search, dismiss behavior.
- [ ] 6.2 Tests: palette omits out-of-context and non-palette commands; toggle shows check; selecting invokes handler and closes.

## 7. Migrate registration sites

- [ ] 7.1 Migrate `RootLayout` nav commands to `useCommand(id, handler)` against defs.
- [ ] 7.2 Migrate `hooks/useShellCommands.ts` handlers to `useCommand`.
- [ ] 7.3 Migrate `PanelContent.tsx` command registration to `useCommand`.
- [ ] 7.4 Delete `ACTION_TITLES` and the standalone `PRESETS` once nothing reads them; `ACTION` id constants stay.
- [ ] 7.5 Tests/regression: every previously-registered command still lists, invokes, and resolves its binding.

## 8. Verify

- [ ] 8.1 `bun test` (apps/ui) green; type-check + lint clean.
- [ ] 8.2 Manual smoke: palette lists/filters/invokes; shortcuts fire and respect context; overrides/preset/leader still work; a sample toggle command shows checked state.
