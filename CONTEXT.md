# tillerd

A local-first desktop workspace that runs terminal (and later agent/diff) surfaces, supervised by a Rust orchestrator over a single daemon and gate.

## Language

### Workspace

**Project**:
A named workspace root that owns a default launch template and groups sessions.
_Avoid_: workspace, repo

**Session**:
A container instantiated from a project's launch template; owns its launch spec, its surfaces, and its panel tree. The launch spec may diverge from the template per session.
_Avoid_: tab, window

**Launch spec**:
The per-session declarative authority for which surfaces a session has and where they land. An ordered list of launch items; the source of truth for surfaces (not the panel tree).
_Avoid_: layout, config

**Launch item**:
One entry in a launch spec: `{ target (surface kind), placement, command }`. Producing a surface is what a launch item does.

**Surface**:
The running leaf a launch item produces; kind-tagged (`terminal`, `diff`). Belongs to exactly one session. A terminal surface is a daemon PTY. A surface is not a panel.
_Avoid_: pane, panel, terminal-as-noun

**Placement**:
The named region that binds a launch item's surface to a slot in the panel tree (e.g. `center` / `side`). The seam between a surface and where it renders.

### View

**Panel**:
A geometry slot (leaf) in a session's panel tree. Renders a surface by placement; it never owns the surface.
_Avoid_: surface, pane

**Panel tree**:
A session's geometry only: splits and tabs of panels. Binds surfaces by placement; carries no surface ownership.
_Avoid_: layout-as-authority

**Chrome**:
App-shell UI that is not a surface and not session-scoped (sidebar, host-status badge). Outside the surface model.
_Avoid_: panel, surface

### Identity

**surface_id**:
The single id tying a surface across processes: the renderer's pane, the orchestrator's proxy, and the daemon PTY. Equals the correlation id.
_Avoid_: pty-id, terminal-id

**session_id**:
The session container id. Product-only; never leaves the orchestrator. Distinct from surface_id.
