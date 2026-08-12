## Context

Tillerd currently drives coding agents through the user's own login while the desktop
app owns workspace chrome and sessions own surfaces and their panel trees. The
`agent-view-runtime` change adds a way for an agent to present local HTML or Markdown
in the workbench right dock, to request local diff data, and to publish a durable
review. It must not turn an agent view into a surface or give untrusted content a path
to the desktop runtime, filesystem, gateway control plane, or another session.

The proposal requires a viewer substrate first, then diff data, review UI, and finally
interactive views. The existing MCP gateway remains the standard, loopback-only MCP
front. A per-session token continues to scope agent authority; it is never delivered
to rendered content. Reviews and findings are local operational data, while the
workspace, project, session, launch spec, surface, and placement contracts remain
unchanged.

## Goals / Non-Goals

**Goals:**

- Render agent-supplied HTML and Markdown safely in a right-dock chrome view.
- Make the host-mediated bridge the only communication path between rendered content
  and Tillerd, with explicit per-view capability grants.
- Provide project-rooted local diffs and durable reviews with inline findings.
- Keep all authority session-scoped, observable, bounded, and revocable on view close.
- Add only the persistence required for reviews and findings.

**Non-Goals:**

- Adding an agent surface, launch item, placement, PTY proxy, or changes to the panel
  tree.
- Changing the gateway transport, service lifecycle, wire framing, ACL model, or
  public control-plane shape.
- Network access, arbitrary file access, arbitrary process execution, or direct Tauri
  access from rendered content.
- Synchronizing review data, multi-user collaboration, remote-hosted views, or applying
  suggestion patches without an explicit user action.

## Decisions

### Isolated view document

The right dock hosts each view in an iframe with an opaque origin and
`sandbox="allow-scripts"`. The document is delivered inline from the host; it has no
network base URL and receives neither `allow-same-origin` nor any navigation, popup,
form, download, pointer-lock, or top-level access permission. Markdown is rendered by
the host into the same isolated document rather than granting a renderer plugin access
to the app document.

The iframe document receives a strict per-view CSP: default deny; scripts limited to
the bundled view runtime; styles limited to the bundled renderer; images limited to
explicitly provided inline content; and no connections, frames, workers, media, fonts,
forms, or navigation. The Tauri webview CSP is set explicitly as a separate outer
boundary and permits only the desktop app's required bundled resources. It does not
relax the iframe policy.

This is smaller and safer than a same-origin iframe with a sanitization-only policy, or
than a custom webview per agent view. Sanitization still protects the host renderer,
but the sandbox and CSP contain a missed sanitizer case.

### Capability-gated bridge

The parent creates one short-lived bridge endpoint for each view instance. A bridge
message includes the instance identity, a monotonically increasing request id, and a
declared capability; the parent accepts it only from that iframe window and only while
the instance is active. An identifier must be strictly greater than the last accepted
identifier for that instance; replayed or out-of-order identifiers are rejected before
the capability executes. The opaque origin is intentional, so `event.source` and the
per-instance identity, rather than an origin string, bind the endpoint. The host
validates every payload with the project's normal validation boundary and returns
typed success or error responses. Messages are size-limited and rate-limited; closing
or replacing a view revokes the endpoint and rejects outstanding requests.

Initial capabilities are declarative and narrow: render lifecycle updates, read-only
diff retrieval, and review publication. Interactive intents are a later increment and
must be individually granted by the host for the same session; they cannot mint another
capability, access a different session, or invoke management operations. The per-session
token is verified at the existing agent-control boundary and remains host-only. The
iframe receives only its bridge endpoint, never a bearer token or a generic MCP client.

This keeps one mediated path instead of exposing Tauri APIs, `postMessage` handlers
with action names, or a generic RPC bridge. It also preserves the gateway's standard
MCP transport and the separation between agent tools and daemon administration.

### Diff and review contracts

Diff requests name one project and one bounded target: working tree, staged changes, or
an explicit commit range. For an agent operation, the host derives project authority
from the verified session and rejects a mismatched project before repository access.
The host resolves the project root from the authorized project record, rejects paths
outside it, and returns a structured, read-only diff model made
of files, hunks, and line mappings. The UI's vendored diff renderer consumes that model;
the bridge never receives a filesystem handle or a shell command. Missing repositories,
invalid revisions, oversized output, and unavailable diffs are typed failures rather
than partial authority grants.

A published review contains its target, summary, walkthrough, and status. Its findings
are anchored to a project-relative file and range, with severity, category, body, state,
and an optional suggestion patch. The host checks that anchors and suggestions belong to
the reviewed target before persistence. The review UI can filter, accept, or dismiss a
finding. Applying a suggestion is a separate user-initiated operation against the
current working tree after the UI rechecks the target; agent publication never writes a
file by itself.

Structured contracts are preferred to raw repository or patch access because they make
scope, validation, rendering, and persistence explicit while reusing the existing
application-layer domain logic.

### Persistence and placement

Reviews and findings use the proposal's additive local migration in the operational
`state.db`, keyed by the existing stable workspace, project, and session identities as
applicable. Review state is durable across app restart; runtime bridge state, rendered
HTML, capability grants, and open dock selection are not durable authority and are
discarded when their view closes or the app exits. The migration does not modify the
readable workspace snapshot tree, launch spec, panel tree, surface binding, or placement
schema.

Agent views are right-dock chrome. The dock may show a diff or review view, but it does
not claim a placement, create a surface, or alter the session's surface ownership. UI
commands for opening or closing the dock are contributions to the existing command
model rather than new ad-hoc shortcuts or toolbar handlers.

Using the operational plane for review data follows its role as machine-local typed
view state and avoids coupling a review to a relocatable workspace snapshot. Persisting
view runtime state or introducing a new domain-plane entity would add recovery and
migration obligations without a proposal requirement.

### Incremental delivery and observability

Implementation lands in proposal order: isolated read-only viewer, read-only diff,
durable review UI, then interactive capability grants. Each boundary logs the session,
project, view instance, capability, operation, and typed outcome without logging raw
rendered content or tokens. Operations have bounded payloads, timeouts, cancellation on
view close, and independent failure handling so a failed view or diff does not stop a
session or its terminal surface.

This order puts the highest-risk authority boundary last and gives the first increments
useful, reviewable behavior without expanding the frozen seams.

## Risks / Trade-offs

- [Sandbox escape or CSP regression] -> Keep the iframe opaque-origin with scripts only,
  test the generated CSP and sandbox attributes, and treat any new iframe permission as
  a security review.
- [Bridge confusion between views or sessions] -> Bind requests to one live iframe
  instance and host-side session scope; revoke endpoints eagerly and validate every
  payload.
- [Diff output is too large or stale] -> Bound output, return typed size or revision
  errors, and recheck the target before applying a user-approved suggestion.
- [Durable findings become detached from changed code] -> Preserve the reviewed target
  and file/range anchors; show stale or unresolved anchors as review state, never apply
  them automatically.
- [Capability scope expands by convenience] -> Start read-only, name each grant, and
  require a separate design and security review before adding write or network authority.
- [Schema migration blocks downgrade] -> Keep review/finding tables additive and
  ignored by an older app; do not migrate existing workspace or surface data.

## Migration Plan

1. Add the outer and per-view CSP policies together with the isolated, read-only dock
   viewer; verify that a view has no direct desktop, network, or cross-session access.
2. Add the project-rooted diff contract and reuse the existing diff renderer without
   changing surface or placement records.
3. Add the additive operational-state migration and review/finding UI, including
   anchor validation and explicit user approval for suggestion application.
4. Add interactive bridge capabilities only after the read-only flow has exercised
   endpoint revocation, typed errors, limits, and session scoping.

Rollback hides the dock entry points and rejects new bridge requests. Existing additive
review records remain local and inert; no workspace, launch-spec, placement, service,
wire, or ACL rollback is required.

## Open Questions

- Which interactive-UI extension details can be adopted without making the MCP gateway
  expose a non-standard transport or public protocol?
- What bounded size and rate limits preserve useful rich views while satisfying the
  reliability contract on supported macOS and Linux hardware?
- Should persisted reviews be associated only with a project and target, or also retain
  the publishing session when that session is later archived?
- What exact user-confirmation UX is required for applying a suggestion patch, including
  conflicts and a changed working tree?
- No in-force ADR requires supersession for this design. The ADR artifact should record
  the final iframe, CSP, bridge, and capability-grant boundary before implementation;
  it must not modify the frozen service, wire, ACL, data-model, or placement seams.
