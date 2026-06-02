## 1. Dependencies & Tooling

- [x] 1.1 Verify `@tailwindcss/vite` in `apps/ui/package.json`; add if missing and configure plugin in `vite.config.ts`
- [x] 1.2 Verify `@fontsource-variable/geist` and `tw-animate-css` installed (shadcn init should have added them — `bun install` if not)
- [x] 1.3 Add `@pierre/diffs` and `lucide-react` to `apps/ui/package.json`
- [x] 1.3 Run `npx shadcn@latest init -b base --template react-router -y -c apps/ui` — initializes shadcn with Base UI, creates `components.json`
- [x] 1.4 Run `npx shadcn@latest add resizable tooltip tabs scroll-area separator -c apps/ui` — copies styled Base UI wrappers into `apps/ui/app/components/ui/`
- [x] 1.5 Define compact design tokens in `apps/ui/app/app.css`: `@import "tailwindcss"` + override shadcn CSS vars (`--radius: 0`, `--font-size-sm: 12px`, custom `--color-bg`, `--color-panel-header`)

## 2. Server — Diff Endpoint

- [x] 2.1 Add `GET /api/sessions/:id/diff` to `apps/server/src/index.ts`
- [x] 2.2 Resolve session `cwd` from SQLite; return 404 JSON if not found
- [x] 2.3 Spawn `git diff HEAD` via `Bun.spawn`; stream stdout as `text/plain`
- [x] 2.4 Handle non-git directory gracefully (empty 200)
- [x] 2.5 Apply CORS headers; handle OPTIONS preflight

## 3. Router Restructure

- [x] 3.1 Create `apps/ui/app/routes/_shell.tsx` as pathless layout route
- [x] 3.2 Add `clientLoader` to `_shell.tsx`: `fetch('/api/sessions')` → `loaderData.sessions`
- [x] 3.3 Create `apps/ui/app/routes/_shell._index.tsx` — spawning page: mounts bare `TerminalPane` (no sessionId), receives `session_start`, calls `useNavigate("/session/" + id)`
- [x] 3.4 Create `apps/ui/app/routes/_shell.session.$id.tsx` — provides sessionId to panel context
- [x] 3.5 Update `apps/ui/app/routes.ts` using `layout()` + `route()` helpers from `@react-router/dev/routes`
- [x] 3.6 Remove `apps/ui/app/routes/_index.tsx`

## 4. Panel Tree Model

- [x] 4.1 Define types: `PanelNode`, `PanelGroupNode`, `PanelLeaf`, `PanelContent`, `ToolbarConfig` in `apps/ui/app/lib/panelTree.ts`
- [x] 4.2 Implement `DEFAULT_LAYOUT`: horizontal split group — Sessions sidebar, Terminal, Changes diff
- [x] 4.3 Implement `serializeLayout` / `deserializeLayout` with try/catch fallback to default
- [x] 4.4 Create `usePanelTree` hook: `tree`, `split(id, direction)`, `setContent(id, content)`, `setDisplayMode(groupId, mode)`, `setActiveTab(groupId, tabId)`
- [x] 4.5 `split`: replaces leaf with `PanelGroupNode { kind:'group', direction, displayMode:'split', children:[originalLeaf, newEmptyLeaf] }`
- [x] 4.6 `close(id)`: removes leaf from parent group; if parent group has 1 child remaining, replaces group with that child (group collapse); no-op if shell has exactly 1 panel
- [x] 4.7 Write unit tests: `split`, `close` (including group-collapse case), `setContent`, serialize/deserialize round-trip, corrupt-data fallback

## 5. Panel Compound Component (`Panel.*`)

- [x] 5.1 Create `apps/ui/app/components/Panel/index.ts` — exports `Panel` namespace
- [x] 5.2 Define `PanelContext` with `{ state: { id, title }, actions: { split, close }, meta: {} }`; `PanelProvider` as the only writer
- [x] 5.3 `Panel.Frame`: outer `div`, flex column, full height, `data-panel-id` attribute
- [x] 5.4 `Panel.Header`: 24px flex row, `var(--color-panel-header)` bg, `var(--border-panel)` bottom border
- [x] 5.5 `Panel.Title`: reads `state.title` from context via `use(PanelContext)`; truncated `span`
- [x] 5.6 `Panel.Toolbar`: right-aligned flex row inside `Panel.Header`
- [x] 5.7 `Panel.Toolbar.Button`: wraps shadcn `<Tooltip>` (from `~/components/ui/tooltip`); props: `icon: ReactNode`, `label: string`, `onClick: () => void`; keyboard-focusable; no boolean props
- [x] 5.8 `Panel.Content`: `div`, `flex: 1`, `overflow: hidden`
- [x] 5.9 Add close button to `Panel.Header` (always visible, `×` icon, calls `actions.close`); no close on the last remaining panel in the shell
- [x] 5.10 Verify: no `forwardRef`, all sub-components defined at module level (not inside other components), `use()` not `useContext()`

## 6. PanelGroup Compound Component (`PanelGroup.*`)

- [x] 6.1 Create `apps/ui/app/components/PanelGroup/index.ts` — exports `PanelGroup` namespace
- [x] 6.2 Define `PanelGroupContext` with `{ state: { displayMode, activeTabId, direction }, actions: { setActiveTab } }`
- [x] 6.3 `PanelGroup.Split`: renders shadcn `<ResizablePanelGroup>` + `<ResizablePanel>` + `<ResizableHandle>` (from `~/components/ui/resizable`); `autoSaveId` keyed by group path
- [x] 6.4 `PanelGroup.TabBar`: uses shadcn `<Tabs>` (from `~/components/ui/tabs`) as the outer container; position (top/bottom) derived from `displayMode` in context, NOT a prop
- [x] 6.5 `PanelGroup.TabBar.Tab`: renders as a shadcn `<TabsTrigger>`; accepts `panelId: string` and `title: string` from `renderNode`; active state from `activeTabId` in context
- [x] 6.6 `PanelGroup.Sidebar`: vertical flex list of sidebar items
- [x] 6.7 `PanelGroup.Sidebar.Item`: accepts `panelId: string` and `title: string` as props (passed by `renderNode`); expands/collapses on click; chevron indicator
- [x] 6.8 `PanelGroup.Panels`: in split mode renders all children; in tabbar/sidebar modes renders only the active panel
- [x] 6.9 Verify: no boolean props on any sub-component; mode variants are different sub-component compositions, not conditionals inside one component

## 7. AppShell Component

- [x] 7.1 Create `apps/ui/app/components/AppShell.tsx` — reads `tree` from `usePanelTree`, renders recursively via `renderNode(node)`
- [x] 7.2 `renderNode(PanelGroupNode)`: selects composition based on `displayMode` (Split, TabBar, Sidebar) — no boolean conditionals
- [x] 7.3 `renderNode(PanelLeaf)`: wraps in `Panel.Provider`, renders `Panel.Frame` + `Panel.Header` + content component
- [x] 7.4 Content component dispatch: `{ sidebar → SessionSidebar, terminal → TerminalPane, diff → DiffPanel, empty → EmptyPanel }`
- [x] 7.5 Read `sessionId` from React Router `useParams` in `AppShell`; pass into `terminal`/`diff` leaf panels via `SessionContext`

## 8. Session Sidebar Panel Content

- [x] 8.1 Create `apps/ui/app/components/SessionSidebar.tsx`
- [x] 8.2 Receives `sessions` as prop (from `loaderData` via `AppShell`); no internal fetch
- [x] 8.3 Wraps list in shadcn `<ScrollArea>` (from `~/components/ui/scroll-area`); rows are `<NavLink>` (28px, gets `isActive` for free)
- [x] 8.4 Row content: color-dot status indicator + truncated session ID (8 chars) + cwd basename
- [x] 8.5 Empty state when `sessions.length === 0`
- [x] 8.6 "New session" button at top: navigates to `/` (WS spawns on connect)

## 9. Terminal Pane Panel Content

- [x] 9.1 Create `apps/ui/app/components/TerminalPane.tsx` — accepts `sessionId: string | null`
- [x] 9.2 Dynamically import `@xterm/xterm` and `@xterm/addon-fit` (`bundle-dynamic-imports` rule — xterm is heavy)
- [x] 9.3 Extract xterm init from old `_index.tsx`; connect to `/ws/session?id=<id>` or bare `/ws/session`
- [x] 9.4 Handle WS messages: `session_start`, `session_resume`, `data`, `status`, `exit`
- [x] 9.5 `ResizeObserver` → `fitAddon.fit()` → send resize message
- [x] 9.6 Tear down on `sessionId` change or unmount
- [x] 9.7 Add `onSessionStart: (id: string) => void` prop — called when WS `session_start` fires; used by `_index.tsx` to navigate to `/session/:id`
- [x] 9.8 Call `useRevalidator().revalidate()` on `session_start` and `exit` WS events to refresh sidebar `clientLoader` data
- [x] 9.9 Toolbar buttons (via `Panel.Toolbar.Button` in parent `Panel.Header`): interrupt ⌃C, reconnect — wired via context actions, not props drilled into `TerminalPane`

## 10. Diff Panel Content

- [x] 10.1 Create `apps/ui/app/components/DiffPanel.tsx` — accepts `sessionId: string | null`
- [x] 10.2 Subscribe to session status via `SessionContext`; fetch on IDLE/DONE transition
- [x] 10.3 Parse unified patch with `@pierre/diffs` utilities; wrap in `WorkerPoolProvider`
- [x] 10.4 Render files in `<Virtualizer>` + `<FileDiff>` with dark theme
- [x] 10.5 Toolbar button for stacked/split toggle (rendered in parent `Panel.Header` via context actions)
- [x] 10.6 Loading skeleton; empty state; not-a-git-repo message; null-session placeholder

## 11. Empty Panel Content

- [x] 11.1 Create `apps/ui/app/components/EmptyPanel.tsx`
- [x] 11.2 Renders content-type picker: sidebar, terminal, diff options
- [x] 11.3 On selection: calls `usePanelTree().setContent(panelId, { type })`

## 12. Session Context & Status Signal

- [x] 12.1 Create `apps/ui/app/lib/sessionContext.ts`: `SessionContext` with `{ sessionId: string | null, status: string, setStatus: (s: string) => void }`; `status` is React state (not a ref)
- [x] 12.2 `AppShell` owns `SessionContext.Provider` — reads `sessionId` from `useParams()`, holds `useState` for `status`; wraps sidebar + Outlet + DiffPanel together so all three can access context (DiffPanel is outside Outlet — provider must be above both)
- [x] 12.3 `TerminalPane` (inside Outlet) calls `setStatus` from context when WS `status` message arrives
- [x] 12.4 `DiffPanel` (outside Outlet, in AppShell) reads `status` from same context; `useEffect([status])` fires fetch when `status === 'IDLE' || status === 'DONE'`

## 13. E2E Tests (Playwright — `tests/e2e/tests/`)

- [x] 13.1 Create `shell.spec.ts` — default layout: three columns visible, resize handle draggable, width persists after reload
- [x] 13.2 `shell.spec.ts` — panel split: click split-H button on terminal panel → two panels appear side by side
- [x] 13.3 `shell.spec.ts` — panel close: click × on a panel → panel removed; single remaining panel fills space
- [x] 13.4 Create `session.spec.ts` — golden path: load app → "New session" → terminal connects → session row appears in sidebar → sidebar row is active
- [x] 13.5 `session.spec.ts` — navigate between two sessions → terminal content switches → active sidebar row updates
- [x] 13.6 `session.spec.ts` — diff panel: mock `GET /api/sessions/:id/diff` with fixture patch → trigger IDLE status → diff panel renders file entries
- [x] 13.7 Audit and update stale specs: `dashboard.spec.ts`, `navigation.spec.ts`, `smoke.spec.ts`, `integration.spec.ts` — remove or rewrite tests for routes removed when `root.tsx` nav was stripped
- [x] 13.8 Verify `tests/e2e/playwright.config.ts` `baseURL` matches dev server port (`5173` — confirm against `bun run dev` output)

## 14. Manual Verification

- [ ] 14.1 `bun run dev` starts without errors (server + UI)
- [ ] 14.2 Tabbar-top mode: tabs render titles; only active panel mounts
- [ ] 14.3 Sidebar mode: vertical list renders; click expands correct panel
- [x] 14.4 `bun run check-types` passes; no new type errors
