## Why

`apps/ui` is a single-file terminal dump with no session management, no layout, and no visibility into what the agent changed. A proper shell — flexible recursive panel layout, per-session terminal, and diff panel — makes the reference UI actually usable and demonstrates the SDK's full session lifecycle.

## What Changes

- Replace the single-route terminal page with a layout shell built on a recursive panel tree
- Every panel group supports four display modes: `split` (resize handles), `tabbar-top`, `tabbar-bottom`, `sidebar` (accordion)
- Every panel has a required title; every panel optionally has a toolbar with icon buttons
- All new UI components follow compound component architecture (shared context, no boolean props, React 19 `use()`)
- Session list fetched via React Router `clientLoader` in layout route; `NavLink` for session rows
- Extract xterm terminal into a reusable `TerminalPane` panel placed anywhere in the layout tree
- Add diff panel that fetches `GET /api/sessions/:id/diff` and renders syntax-highlighted file diffs when session reaches IDLE/DONE
- Add `GET /api/sessions/:id/diff` endpoint to `apps/server`
- Adopt Tailwind v4 (`@tailwindcss/vite`); compact, dense editor aesthetic
- Initialize shadcn (`--base base`) with Base UI primitives: `resizable`, `tooltip`, `tabs`, `scroll-area`, `separator`
- Add `@pierre/diffs` (Apache-2.0), `lucide-react`; `@base-ui/react` and `react-resizable-panels` arrive transitively via shadcn

## Capabilities

### New Capabilities

- `ui-shell`: Recursive panel tree layout; panel groups with four display modes; default three-column layout
- `ui-panel-compound`: Compound component API — `Panel.*`, `PanelGroup.*`, `Panel.Toolbar.*` with shared contexts, no boolean props
- `ui-panel-model`: Layout state model — `PanelGroupNode` / `PanelLeaf` tree with `title`, `displayMode`, `toolbar`; serialize/deserialize to `localStorage`
- `ui-session-sidebar`: Session list panel content — renders compact rows via `NavLink`, `clientLoader` in shell route
- `ui-terminal-pane`: Reusable xterm panel content — per-session WS, resize, status indicator, toolbar buttons
- `ui-diff-panel`: Diff viewer panel content — `@pierre/diffs`, fetches on IDLE/DONE, stacked/split toggle via toolbar button
- `session-diff-endpoint`: `GET /api/sessions/:id/diff` — resolves `cwd` from SQLite, runs `git diff HEAD`, returns unified patch

### Modified Capabilities

- `agent-session`: Session WebSocket consumed by `TerminalPane` (per-session, not global); no protocol change

## Impact

- `apps/ui`: routing restructure (`_shell` layout), new component library, Tailwind v4, three new npm deps
- `apps/server/src/index.ts`: one new HTTP route
- No changes to `@athing/sdk`, `@athing/engine`, or any adapter
- No wire protocol changes
