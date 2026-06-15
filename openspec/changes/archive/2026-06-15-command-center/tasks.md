## 1. Keybinding core (renderer, pure)

- [x] 1.1 Add `cmdk` to `apps/ui` (`bun add cmdk`); generate shadcn `components/ui/command.tsx`. Add `keybindings.preset` + `keybindings.overrides` keys to `apps/ui/app/lib/settings/keys.ts`.
- [x] 1.2 TDD the preset tables (`default` full; `vim`/`vscode`/`tmux` for wired action ids) and the binding parse/format helpers (canonicalize, render).
- [x] 1.3 TDD the keybinding resolution module: preset baseline → per-action override merge, clear-to-fallback, absent-from-preset → no binding.

## 2. Action registry + leader-key port

- [x] 2.1 TDD the action registry: typed entries (`id`, `title`, `keywords?`, `run`), assembly over the live shell callbacks, availability derivation. Lift existing inline handlers (sidebar new-session; panel split-h/-v/close/detach; open-in-new-window; session.switch; view.logs; app.settings) into entries and route the original controls through registry `run()`.
- [x] 2.2 Define the `LeaderKeyPort` (`onActivate`, `setBinding`) + desktop adapter; wire the renderer listener for `command-center:open`.
- [x] 2.3 Register the leader native menu accelerator in `apps/desktop/src-tauri/src/lib.rs` (alongside `View > Logs`), emitting `command-center:open`; default `CmdOrCtrl+K`, driven by the leader setting. Add the Rust host test asserting registration + event-id mapping.

## 3. Command palette overlay + wiring

- [x] 3.1 TDD + build the palette overlay (shadcn `Command`): lists available actions, fuzzy-filters on query, renders each action's resolved binding hint, Enter invokes + closes, Escape/outside dismisses without invoking.
- [x] 3.2 Wire it together: leader (port) opens the overlay; in-renderer shortcut listener fires an action's resolved binding only when no terminal surface holds focus; preset selection + per-action override UI persists via `useGlobalSetting`.

## 4. E2E + verify gate

- [x] 4.1 Desktop e2e (`tests/desktop-e2e/`): open the palette by emitting `command-center:open`, assert fuzzy filter + action invocation + override persists across reload (shared-`TILLERD_DIR` isolation, unique-name targeting, real-DOM-event dispatch per the testing memory).
- [x] 4.2 Final gate: run `bun run verify` (format:check + check-types + lint + test + e2e); fix all failures; confirm every spec scenario maps to a passing test (`/opsx:verify`).
