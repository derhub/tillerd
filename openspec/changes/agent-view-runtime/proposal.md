# Proposal: agent-view-runtime

## Why

The app's differentiating capability is letting the coding agent running in a terminal
drive the app's own UI: open rendered content beside the terminal, present a structured
code review, and ultimately ship small interactive views of its own. Today the agent can
only print text; rendered output, diffs, and review workflows all leave the app. A
sandboxed view runtime plus a local diff/review pipeline keeps that loop local-first and
makes the app more than a terminal host.

## What Changes

- **Agent view runtime (substrate)** — a sandboxed renderer for agent-supplied content
  (html and markdown first) hosted in a right-dock view container: opaque-origin
  sandboxed iframe (`allow-scripts` only, no same-origin), strict per-view CSP, content
  delivered inline (no network), a host-mediated message bridge as the sole channel, and
  a hardened app-level webview CSP (currently unset). Aligned with the emerging MCP
  interactive-UI extension so third-party-served views can adopt the same shape later.
- **Agent control plane** — first-party tools exposed through the in-repo MCP gateway
  (today aggregation-only; gains a first-party tool path): open/update/close a view,
  publish a review, fetch diff data. Token-scoped per session, mirroring the existing
  hook-ingress trust model.
- **Diff API + diff view** — a local diff pipeline (working tree / staged / commit
  range, sourced from the project's repository) rendered by the already-vendored diff
  renderer; the orphaned diff panel component returns to service as a dock/panel view.
- **Review model + review UI** — a persisted review data model (review: target, summary,
  walkthrough, status; finding: file, range, severity, category, body, optional
  suggestion patch, state) with a review view: summary block, per-file walkthrough,
  inline findings on the diff, severity filtering, accept/dismiss per finding, apply for
  suggestion patches. Reviews are produced by the agent through the control plane and
  live entirely locally.
- **Interactive app views (final increment)** — agent-authored interactive views with a
  bidirectional bridge (view→host intents behind explicit capability grants), building
  on the same substrate; highest risk, lands last.

## Capabilities

### New Capabilities

- `agent-view-runtime`: sandboxed rendering of agent-supplied html/markdown views in a
  dock container — delivery, isolation, lifecycle, message bridge.
- `agent-control-plane`: first-party MCP tools for view/review/diff operations,
  per-session token scoping.
- `code-diff-api`: local diff computation over a project repository (working tree,
  staged, ranges) exposed to both the UI and the control plane.
- `code-review-ui`: review + finding model, persistence, and the review view over the
  diff renderer.

### Modified Capabilities

- `ui-diff-panel`: orphaned component becomes the diff view fed by `code-diff-api`
  (split/unified, syntax highlighting, virtualization retained).
- `mcp-gateway-aggregation`: gateway gains a first-party tool path alongside backend
  aggregation.
- `ui-workbench` (from the ux-ui-overhaul change): right dock returns as the agent-view
  container.

## Impact

- New: view-runtime renderer + bridge in `apps/ui`; review/finding tables (additive
  migration) + diff/review operations in the orchestrator app layer; first-party tool
  registration in `apps/mcp-gateway`.
- Modified: workbench right dock; Tauri webview CSP set app-wide (security posture
  change, currently null); transport additions (additive; wire protocol and ACL model
  unchanged).
- Sequencing: depends on the ux-ui-overhaul workbench; increments land risk-ascending —
  viewer substrate → diff API → review UI → interactive views. 0.x remains
  terminal-only: these are chrome views, not session surfaces (no change to the
  surface model or ADR-0027).
- Security: this change introduces rendering of untrusted agent-authored content; the
  ADR records the isolation model (sandbox flags, CSP, bridge protocol, capability
  grants) before any implementation.
