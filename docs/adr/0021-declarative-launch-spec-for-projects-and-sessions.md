# 0021. Projects and sessions are instances of a declarative launch spec

- Status: proposed
- Date: 2026-06-11
- Amended by: ADR-0030 (placement is a unique slot id, not a fixed `center/side` set)

## Context

ADR-0020 established that a desktop session is a container of surfaces, and a
surface is a kind-tagged leaf (`terminal`, `agent`, and later others). That model
says *what a session is* but not *how one comes to exist*. The desktop needs a
workspace flow: open the app, register a project, create a session, and have its
surfaces start with the right commands in the right places — a terminal running a
dev server on one side, an agent in a worktree on the other.

Two gaps remain:

1. No **project** concept exists. The desktop has sessions but nothing groups them
   or binds them to a working tree.
2. Session creation is imperative and fixed. There is no declarative way to say
   "this session starts these surfaces, with these commands, env, and pre/post
   steps."

The requirement is flexibility without per-feature rework: today a login shell and
an agent CLI; tomorrow multi-script dev setups, worktree-isolated agent runs, and
containerized execution. The cost of getting the *schema* wrong is rewritten
surface code every time a workflow is added. So the foundation must be a complete
launch specification with explicit extension seams, even though the first release
ships only a minimal command library — minimal library, total schema.

## Decision

Introduce three desktop/SDK concepts on top of ADR-0020, plus a versioned launch
spec underneath them.

### Project

A named workspace root and the owner of a default launch template.

- Source: one of `blank`, `local-dir`, `git-repo`, `git-worktree`.
- Name: inferred from the source (directory basename, repository name, or branch)
  or from the agent session title; always user-overridable and renamable.
- A project references shared singleton services by id (ADR-0020); it does not
  start them.

### Launch spec

A versioned, declarative description of what a session starts: an ordered list of
**launch items**. Each item:

| Field | Meaning |
| --- | --- |
| `target` | surface kind to create (an ADR-0020 kind: `terminal`, `agent`, ...) |
| `placement` | where the surface lands in the layout (see Placement) |
| `command` | a command-library reference, or an inline `{ cli, args, env }`; default is a login shell |
| `pre` / `post` | scripts run before the command starts / after it exits |
| `autoSpawn` | background scripts started alongside (dev setup) |
| `worktree` | optional: create a worktree, `cd` into it, then run the command |

The spec is the contract. New workflows are new data (items plus library entries),
not new code.

### Command library

A catalog of named commands that launch items reference: prebuilt entries (login
shell, agent CLI presets) plus user-defined entries (`cli + args + env`). The first
release ships a minimal prebuilt set; the catalog is the extension point for
bespoke and, later, prebuilt workflow commands.

### Template versus instance

- A **project** owns a default launch template (a launch spec).
- A **session** is an instance created from that template; it may diverge
  (add / remove / edit items) without changing the project default.
- A **surface** is the running leaf a launch item produces.
- Session title is inferred (agent session title or branch name, user choice or
  both) and customizable.

### Placement

Placement is where a launch item's surface lands in the session layout. The panel
tree already supports arbitrary splits and tabs (split / tabbar / sidebar).
Placement is a named seam: the first release uses a minimal **named-region** model
(for example `center` / `side`) mapped onto the panel tree; exact geometry (sizes,
nested splits) is refined per release. The launch spec carries a `placement` field
from day one so later geometry work is additive.

### Lifecycle and archival

- Creating a session or surface creates the corresponding per-run records in the
  already-running services (ADR-0020); it never starts a service.
- Delete is **archive**, not removal: an archived session is recoverable, and its
  worktree is kept, not deleted. A separate hard-delete acts only on archived items
  and is unrecoverable.

### Persistence and execution

- Projects, templates, the command library, and sessions persist as durable state
  in SQLite (the project persistence standard); ephemeral runtime state and service
  discovery stay out.
- The execution backend is an extension seam: launch items run as local processes
  now; containerized backends (dev-container spec, OCI runtimes) slot in behind the
  same launch-item contract in a later release.
- Pre / post / auto-spawn scripts run unsandboxed — this is a local-trust developer
  tool. Sandboxing is explicitly out of scope and the lowest-priority follow-up.

## Consequences

- The desktop gains a project layer above the ADR-0020 session; the runtime
  topology (shared singleton services referenced by id) is unchanged.
- Workflow flexibility becomes a data concern: the launch spec plus command library
  absorb dev-server setups, worktree-isolated agent runs, and future presets with
  no surface-code change.
- Placement, surface kind, command library, and execution backend are explicit
  extension seams; the spec is versioned so each evolves additively.
- Archive-over-delete makes destructive actions recoverable by default; hard-delete
  is a deliberate second step.
- New persistence lands on SQLite, advancing the persistence-standard foundation
  item rather than adding a bespoke store.
- The decision constrains the 0.x implementation but ships no code itself. Rollback
  is reverting this file.
