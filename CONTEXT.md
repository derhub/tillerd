# tillerd

A local-first desktop workspace that runs terminal (and later agent/diff) surfaces, supervised by a Rust orchestrator over a single daemon and gate.

## Language

**Workspace**:
A named group of projects that owns its own window; the top of the tree (`workspace → project → session → surface`). A project belongs to exactly one workspace. The built-in Default workspace is un-deletable.
_Avoid_: project, repo

**Project**:
A launch root that groups sessions and carries a source (`sourceKind`, `rootPath`); belongs to exactly one workspace.
_Avoid_: repo

**Session**:
A container created by picking a template from the library (or none — an empty pane); owns a snapshot launch spec, its surfaces, and its panel tree. The spec is decoupled from the template after creation.
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
A per-session-unique slot id, minted by the orchestrator, that binds a launch item's surface to a slot in the panel tree. The boundary between a surface and where it renders. Minted when a surface is added to a session (session creation or a later spawn); a template carries no placement of its own. Unique per session, distinct from surface_id.

### Config

**Profile**:
A portable, named settings bundle (`<profiles>/<name>/settings.jsonc`). Owns settings only — no domain, no templates. One is active and drives the cascade `effective(project) = merge(active profile, workspace override?, project override?)`. Switchable, shareable.
_Avoid_: account, workspace

**Template**:
A portable launch-spec bundle (`<templates>/<slug>/template.jsonc`), sibling to profiles. A library picked from at session creation; the session's spec is a deep-copy snapshot. `prebuilt` (in-code) or `custom` (file). Opt-in and additive — absence or invalidity yields the default empty pane.
_Avoid_: preset, default layout

**Secret**:
A named value stored in the Stronghold vault (`vault.stronghold`, machine-local, encrypted), unlocked by an OS-keychain master password. Referenced by env key in a launch spec; resolved at launch.

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
