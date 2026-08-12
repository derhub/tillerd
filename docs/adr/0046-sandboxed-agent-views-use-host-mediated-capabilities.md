# 0046. Sandboxed agent views use host-mediated capabilities

- Status: accepted
- Date: 2026-08-02
- Supersedes: none

## Context

The `agent-view-runtime` change lets a coding agent render HTML or Markdown in the
desktop right dock, request repository diffs, and publish local code reviews. That
content is untrusted even when it comes from the user's current agent session. It must
not gain ambient access to the host document, desktop runtime, filesystem, network,
gateway control plane, credentials, or another session.

The existing architecture already assigns authority to a verified session token,
keeps the gateway's public face on standard MCP, makes Rust authoritative for state,
and reserves surfaces and placements for session-owned terminal content. Agent views
must fit those boundaries rather than create a parallel runtime or authority model.

## Decision

Agent views SHALL use the following security boundary:

- A view is workbench right-dock chrome. It does not create a surface, claim a
  placement, alter the panel tree, or receive PTY ownership.
- Agent-supplied HTML and host-rendered Markdown run only in an opaque-origin iframe
  with `sandbox="allow-scripts"`. The iframe receives no same-origin, navigation,
  popup, form, download, pointer-lock, top-level, desktop-runtime, filesystem,
  process, or network authority.
- View content is delivered inline. A strict per-view CSP defaults to deny and permits
  only the bundled view runtime, bundled rendering styles, and explicitly supplied
  inline images. Connections, nested frames, workers, media, fonts, forms, and
  navigation remain denied. The desktop webview has a separate explicit outer CSP
  that cannot relax the iframe policy.
- The host creates one short-lived bridge endpoint per live iframe. It binds requests
  to the iframe window and view instance, requires strictly increasing request
  identifiers, validates and bounds every payload, rate-limits operations, and returns
  typed outcomes. Closing or replacing a view revokes its endpoint and rejects pending,
  replayed, and out-of-order requests before they execute.
- Bridge capabilities are explicit and individually granted. Initial authority is
  limited to view lifecycle, read-only diff retrieval, and review publication.
  Rendered content cannot mint capabilities, invoke management operations, access a
  different session, execute an arbitrary process, obtain general filesystem or
  network access, or receive a bearer token or generic MCP client.
- First-party view, diff, and review operations remain on the gateway's existing
  standard MCP face. The gateway verifies the existing per-session token, derives
  project authority from that session, and rejects mismatched session or project
  targets before repository access. This adds no new transport, service lifecycle,
  wire framing, or ACL semantics.
- Diff access is read-only, project-rooted, revision-aware, and bounded. The host
  returns structured files, hunks, and line mappings rather than filesystem handles,
  shell commands, or raw repository authority.
- Reviews and findings are additive local operational state. Target identities and
  file/range anchors are validated before persistence. Applying a suggestion always
  requires explicit user action and a fresh target check; agent publication never
  writes a file by itself.
- Rendered content, open bridge endpoints, capability grants, and dock selection are
  transient authority and are not persisted. Review and finding records may survive a
  restart without changing workspace snapshots, launch specifications, surfaces,
  placements, or their ownership models.

Any additional iframe permission, bridge capability, write authority, network access,
or cross-session operation requires a new security review and a superseding ADR.

## Consequences

- A sanitizer defect alone does not expose the host document because the opaque origin,
  iframe sandbox, and CSP remain independent containment layers.
- Interactive views can grow only through named, host-enforced capabilities; there is
  no generic RPC, desktop API, or MCP client inside rendered content.
- Diff and review workflows remain local and useful while repository and session
  authority stay in existing host boundaries.
- The bridge and CSP require focused negative tests for escape attempts, replay,
  revocation, cross-session and cross-project access, bounds, and stale targets.
- The design deliberately excludes remote views, arbitrary file/process/network access,
  multi-user collaboration, and automatic suggestion application.

