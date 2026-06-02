## Context

`apps/ui` currently has a single route (`_index.tsx`) that opens one hardcoded terminal. There is no session management, no layout flexibility, and no diff visibility. The server exposes `GET /api/sessions` and per-session WebSocket at `/ws/session`; SQLite stores each session's `cwd`.

## Goals / Non-Goals

**Goals:**

- Recursive panel tree: any panel can be split horizontally or vertically
- Four panel group display modes: `split`, `tabbar-top`, `tabbar-bottom`, `sidebar`
- Every panel has a required title; every panel may optionally have a toolbar with buttons
- All panel/group/toolbar components follow compound component architecture
- Compact, dense editor-like aesthetic (VS Code / Zed) — 12px type, 24–28px chrome, 1px borders
- `clientLoader` in layout route for session list; `NavLink` for session rows
- `@pierre/diffs` for diff rendering; `react-resizable-panels` for split mode

**Non-Goals:**

- Drag-and-drop panel reordering
- Diff editing / accept-reject workflows
- Mobile layout
- Plugin/extension system

## Decisions

### D1 — shadcn/ui (`--base base`) + Base UI

shadcn CLI (`npx shadcn init -b base`) initializes shadcn with Base UI as the primitive library instead of Radix UI. Base UI handles all accessibility concerns (ARIA, keyboard, focus management). shadcn provides the styled wrapper components (Tailwind + CSS variables). Components are copied into the codebase — no runtime shadcn dep.

Components added:
- `resizable` — wraps `react-resizable-panels`; handles split mode resize
- `tooltip` — wraps `@base-ui/react/tooltip`; used by `Panel.Toolbar.Button`
- `tabs` — wraps `@base-ui/react/tabs`; used by `PanelGroup.TabBar`
- `scroll-area` — wraps `@base-ui/react/scroll-area`; used by session sidebar
- `separator` — wraps `@base-ui/react/separator`; panel dividers

`react-resizable-panels` arrives transitively via the `resizable` component — not a direct dep. `@base-ui/react` arrives via `shadcn init`.

**Why Base UI over Radix**: React 19 native, no deprecated APIs, no `forwardRef`, matches `use()` patterns. Used by t3code in their sidebar.

### D1a — `react-resizable-panels` (via shadcn Resizable) for `split` display mode only

The shadcn `Resizable` wrapper handles `split` mode. The other three display modes (`tabbar-top`, `tabbar-bottom`, `sidebar`) are pure React — no resize handles, just active-panel state managed in `PanelGroupContext`.

### D2 — Panel tree data model

```typescript
type PanelNode = PanelGroupNode | PanelLeaf

type PanelGroupNode = {
  kind: 'group'
  direction: 'horizontal' | 'vertical'
  displayMode: 'split' | 'tabbar-top' | 'tabbar-bottom' | 'sidebar'
  activeTabId?: string          // for tabbar-* and sidebar modes
  children: PanelNode[]
}

type PanelLeaf = {
  kind: 'panel'
  id: string
  title: string                 // required, shown in header, tabs, sidebar
  content: PanelContent
  toolbar?: ToolbarConfig       // optional list of button configs
}

type PanelContent =
  | { type: 'sidebar' }
  | { type: 'terminal'; sessionId: string | null }
  | { type: 'diff';     sessionId: string | null }
  | { type: 'empty' }

type ToolbarConfig = {
  buttons: ToolbarButtonConfig[]
}
```

Tree stored in React state, initialized from `localStorage`, falls back to default on parse failure. `react-resizable-panels` `autoSaveId` handles split percentages independently.

**Default layout:**
```
PanelGroup(horizontal, split)
  PanelLeaf("Sessions", content: sidebar)
  PanelLeaf("Terminal", content: terminal)
  PanelLeaf("Changes", content: diff)
```

### D3 — Compound component architecture (all panel/group/toolbar components)

Every new component follows the compound component pattern with shared context. No boolean props.

**Pattern** (per `architecture-compound-components` rule):

```tsx
// Each compound has a typed context
const PanelCtx = createContext<PanelContextValue | null>(null)

// Provider injects state + actions + meta
function PanelProvider({ children, state, actions, meta }) {
  return <PanelCtx value={{ state, actions, meta }}>{children}</PanelCtx>
}

// Sub-components use `use()` (React 19 — no `useContext`)
function PanelTitle() {
  const { state: { title } } = use(PanelCtx)
  return <span>{title}</span>
}

// Exported as namespace object
export const Panel = {
  Provider: PanelProvider,
  Frame: PanelFrame,
  Header: PanelHeader,
  Title: PanelTitle,
  Toolbar: PanelToolbar,
  Content: PanelContent,
}
Panel.Toolbar.Button = PanelToolbarButton
```

**No `forwardRef`** (React 19): pass refs as regular props.

**No boolean props**: explicit display mode sub-components for `PanelGroup` instead of `<PanelGroup tabbar />`:

```tsx
// Wrong
<PanelGroup tabbar tabPosition="top" />

// Correct — explicit sub-components, no conditionals
<PanelGroup.Provider displayMode="tabbar-top">
  <PanelGroup.TabBar>
    <PanelGroup.TabBar.Tab panelId="terminal" />
  </PanelGroup.TabBar>
  <PanelGroup.Panels />
</PanelGroup.Provider>
```

### D4 — PanelGroup display modes as explicit sub-component sets

Each display mode has its own sub-components. Consumers compose only what they use:

| Mode | Sub-components | Behavior |
|---|---|---|
| `split` | `PanelGroup.Split` | All children visible, `react-resizable-panels` resize handles |
| `tabbar-top` | `PanelGroup.TabBar` (top) + `PanelGroup.Panels` | One panel visible, tabs above |
| `tabbar-bottom` | `PanelGroup.TabBar` (bottom) + `PanelGroup.Panels` | One panel visible, tabs below |
| `sidebar` | `PanelGroup.Sidebar` + `PanelGroup.Panels` | Vertical accordion of titles; one expanded |

`PanelGroup.TabBar.Tab` reads the panel's `title` from its leaf node to label the tab. `PanelGroup.Sidebar.Item` does the same for the sidebar list.

### D5 — Toolbar compound: `Panel.Toolbar` + `Panel.Toolbar.Button`

Every panel may include an optional toolbar in its header. `Panel.Toolbar` is a flex row in the header's right side. `Panel.Toolbar.Button` renders an icon button with a tooltip label. Both are sub-components of `Panel` — no props passed through the panel to configure them.

```tsx
<Panel.Header>
  <Panel.Title />
  <Panel.Toolbar>
    <Panel.Toolbar.Button icon={<SplitHIcon />} label="Split right" onClick={onSplitH} />
    <Panel.Toolbar.Button icon={<SplitVIcon />} label="Split down" onClick={onSplitV} />
  </Panel.Toolbar>
</Panel.Header>
```

### D6 — React Router `clientLoader` for session list

The `_shell.tsx` layout route uses `clientLoader` to fetch sessions from `GET /api/sessions` (SPA, browser-only). The session list flows as `loaderData` to the `SessionSidebar` panel content. No component-level polling — data is refetched on navigation via React Router's built-in refetch-on-focus / navigate logic.

```tsx
// _shell.tsx
export async function clientLoader(): Promise<{ sessions: Session[] }> {
  const res = await fetch('/api/sessions')
  return res.json()
}

export default function Shell({ loaderData }: Route.ComponentProps) {
  return <AppShell sessions={loaderData.sessions} />
}
```

`SessionSidebar` uses `<NavLink>` (not `<a>`) for session rows — gets `isActive` for free.

### D7 — `@pierre/diffs` for diff rendering

`FileDiff` + `Virtualizer` from `@pierre/diffs/react`. `WorkerPoolProvider` wraps the diff panel content. The diff panel fetches `GET /api/sessions/:id/diff` once on status → IDLE/DONE.

### D8 — Compact aesthetic tokens

CSS variables in `app.css` (Tailwind v4 CSS-first):

| Token | Value |
|---|---|
| `--font-size-base` | 12px |
| `--height-toolbar` | 28px |
| `--height-panel-header` | 24px |
| `--border-panel` | 1px solid #1e1e1e |
| `--radius-panel` | 0px |
| `--radius-button` | 2px |
| `--color-bg` | #0d1117 |
| `--color-panel-header` | #161b22 |
| `--color-text` | #e6edf3 |
| `--color-muted` | #8b949e |

Resize handles: 1px, transparent at rest, `--color-muted` on hover.

## Risks / Trade-offs

- **PanelGroup context + panel tree state**: two separate state layers (tree structure in app state, panel sizes in `react-resizable-panels`). Mitigation: clear ownership — tree state = `usePanelTree`, sizes = `autoSaveId`.
- **`@pierre/diffs` shiki worker**: ~200ms first-render latency. Mitigation: loading skeleton.
- **Tabbar/sidebar modes with `react-resizable-panels`**: these modes bypass `react-resizable-panels` entirely — only `split` mode uses it. Mitigation: conditional rendering, not conditional imports.
- **`clientLoader` revalidation**: React Router revalidates on navigation; session list stays fresh without polling.

### D9 — Session creation via spawning index route

`_shell._index.tsx` is a spawning route, not an empty state. When navigated to, it mounts a bare `TerminalPane` (no `sessionId` prop). The terminal connects to `/ws/session` (no `?id=`), receives `session_start` with the new ID, then calls `onSessionStart(id)` → `useNavigate("/session/" + id)`. The URL becomes canonical only after the session is live.

Server stays stateless — no `POST /api/sessions` endpoint needed.

### D10 — Status signal via React state; provider in AppShell

`SessionContext.Provider` lives in `AppShell`, not in the child route. `AppShell` reads `sessionId` from `useParams()` and holds `useState` for `status`. Both `TerminalPane` (inside `<Outlet>`) and `DiffPanel` (outside `<Outlet>`, sibling in AppShell) are within the provider's subtree — they both access the same context.

`status` is React state (`useState`). `TerminalPane` calls `setStatus` when the WS `status` message arrives. `DiffPanel`'s `useEffect([status])` fires the diff fetch. Re-render cost is acceptable: status changes are infrequent.

### D11 — Tab/sidebar titles via tree, not a registry

`renderNode` has full access to the tree. When rendering a `PanelGroupNode` in tabbar or sidebar mode, `renderNode` passes each child leaf's `{ id, title }` as explicit props to `PanelGroup.TabBar.Tab` / `PanelGroup.Sidebar.Item`. No separate panel registry context needed.

### D12 — Panel close: leaf-only, no group close

Close is a per-leaf action only (no group-level close button). Closing a leaf removes it from its parent group in `usePanelTree`. If the parent group would have only one child remaining after the close, the group is replaced by that single remaining child in the tree (group collapses). The minimum panel count in the shell is 1.

### D13 — Session list revalidation

`TerminalPane` calls `useRevalidator().revalidate()` from the React Router layout route on two WS events: `session_start` (new session was created) and `exit` (session ended). This refreshes `clientLoader` and updates the sidebar without polling.

## Open Questions

- Should `git diff HEAD` or `git diff` (unstaged only)? `HEAD` captures all changes — preferred.
- Should panel display mode be changeable at runtime? Not in scope for v1.
