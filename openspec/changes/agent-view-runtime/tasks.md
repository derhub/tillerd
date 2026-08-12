## 1. Contracts and boundaries

- [ ] 1.1 Define the agent-view, control-plane, diff, review, and right-dock contracts from the seven delta specs, documenting that views are chrome-only and do not create surfaces, launch items, placement records, PTY proxies, or new public wire/ACL seams.
- [ ] 1.2 Add additive review/finding operational-state persistence and migration coverage, leaving workspace, project, session, surface, and placement data unchanged and remaining readable by older app versions.
- [ ] 1.3 Define typed authorization, invalid-path, oversized-result, stale-target, and anchor-validation errors, with session identity and target revision carried only in host-side contracts.

## 2. Isolated view runtime

- [ ] 2.1 Implement the right-dock view document using an opaque-origin sandboxed iframe with scripts only, no network, filesystem, desktop-runtime, navigation, process, or direct Tauri authority.
- [ ] 2.2 Render agent Markdown/HTML inside the isolated document without exposing the host document, host DOM, credentials, or session token to rendered content.
- [ ] 2.3 Generate and apply the strict inner/outer CSP and sandbox attributes at the host boundary; add tests that reject network, navigation, parent-document, filesystem, and desktop-runtime escape attempts.
- [ ] 2.4 Implement per-iframe capability grants and a host-only message bridge with payload validation, strictly increasing request identifiers, typed denial for replayed requests or ungranted capabilities, and no capability expansion through agent content.
- [ ] 2.5 Revoke a view endpoint synchronously when a view closes or is replaced, reject all pending bridge requests, and test that an old iframe cannot use a replacement view or session.
- [ ] 2.6 Add lifecycle, authorization-denial, revocation, oversized-result, and bridge-latency observability correlated to view and session identifiers without logging tokens or raw untrusted content.

## 3. Session-scoped control plane

- [ ] 3.1 Add first-party view, review, and diff operations beside existing aggregated MCP tools while preserving the standard gateway transport, backend aggregation, resource/prompt/tool capability union, and unreachable-backend degradation behavior.
- [ ] 3.2 Verify per-session tokens at the gateway, derive project authority from the verified session, and reject operations naming a different session or project with a typed authorization error; test that rendered views receive results through the bridge without receiving tokens.
- [ ] 3.3 Enforce configured result bounds before returning first-party operation data and return a typed oversized-result error without partial or unbounded content.
- [ ] 3.4 Add control-plane contract tests for authorized and unauthorized view, review, and diff calls, session correlation, timeout/error mapping, graceful shutdown, and degraded gateway operation.

## 4. Bounded project-rooted diffs

- [ ] 4.1 Implement diff retrieval for working-tree, staged, and explicit bounded commit-range targets resolved through the existing project repository authority.
- [ ] 4.2 Canonicalize and validate project-relative paths, reject traversal or symlink escapes with a typed invalid-path error, and test that no path outside the project root is read.
- [ ] 4.3 Return a structured diff containing target identity, files, hunks, and line mappings only; expose no filesystem handle, shell command, arbitrary process capability, or unbounded output.
- [ ] 4.4 Add regression coverage for valid ranges, empty changes, malformed targets, result bounds, stale revisions, raw-byte handling, and macOS/Linux repository behavior.

## 5. Review model and UI

- [ ] 5.1 Implement review and finding publication against a captured target identity, validating every file and range anchor belongs to that target before persistence.
- [ ] 5.2 Persist reviews, findings, anchors, resolution state, suggestions, and target revisions as local operational state; reload them after restart and preserve unresolved or stale anchors without silently rebasing them.
- [ ] 5.3 Require explicit user approval before applying a suggestion, recheck the current target revision, apply only a current suggestion, record the resulting finding state, and reject stale suggestions without automatic patching.
- [ ] 5.4 Connect the structured diff API to the existing diff renderer with loading, empty, error, syntax-highlighted, virtualized, stacked/unified, and side-by-side/split states.
- [ ] 5.5 Add inline finding anchors, unresolved/stale indicators, review resolution controls, and explicit suggestion-approval affordances with keyboard-accessible labels and focus behavior.
- [ ] 5.6 Add review persistence, anchor validation, stale-suggestion, approval, diff rendering, and restart regression tests.

## 6. Workbench right dock

- [ ] 6.1 Restore the right dock as a chrome region for available agent, diff, and review views without creating surfaces or changing placement records.
- [ ] 6.2 Keep sidebar, bottom panel, content area, and right dock visibility independently controlled; verify hidden regions reclaim space and visible regions resize within min/max bounds.
- [ ] 6.3 Add platform-correct native menu accelerator labels and keyboard navigation for dock/view controls, including macOS command-key labels and Linux verification.
- [ ] 6.4 Add UI/E2E coverage for opening, switching, closing, resizing, and restarting the right dock while preserving existing terminal/session lifecycle and panel placement behavior.

## 7. Integration and observability

- [ ] 7.1 Wire view lifecycle, bridge, control-plane, diff, and review operations through existing application composition roots and state authority without changing service lifecycle, wire framing, ACL, data-model, or PTY ownership seams.
- [ ] 7.2 Add structured logs and health/error signals for view/session correlation, rejected capabilities, revoked endpoints, bounded diff failures, stale reviews, and persistence failures while preserving raw-byte and credential handling contracts.
- [ ] 7.3 Verify the additive migration, downgrade tolerance, frozen ADR constraints, no-network/no-arbitrary-process posture, and absence of new surface or placement records in architecture tests.

## 8. Verification and documentation

- [ ] 8.1 Run focused unit and integration tests for sandbox/CSP, bridge revocation, authorization, bounds, path containment, diff structure, review persistence, anchors, suggestion approval, and gateway degradation.
- [ ] 8.2 Run the complete existing test, type, lint, architecture, and ast-grep gates plus the bundled desktop E2E suite with macOS and Linux coverage for lifecycle, dock, terminal, settings, logs, accessibility, and platform chrome.
- [ ] 8.3 Document the implemented contracts, security boundary, capability inventory, persistence format, observability fields, and explicit non-goals, including no remote synchronization or multi-user collaboration.
- [ ] 8.4 Record any non-blocking polish findings as follow-up work and validate the OpenSpec change strictly before implementation is considered complete.
