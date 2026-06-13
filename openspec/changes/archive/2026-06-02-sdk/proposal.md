## Why

Driving coding-agent CLIs (Claude Code first) from an app today means either reimplementing the agent loop against an API key or coupling tightly to a vendor SDK. We want a transport-agnostic SDK that wraps the user's already-installed, already-authenticated CLI as a subprocess — no API key, no vendor SDK — so a future web UI can show the live terminal and rich structured panels on top of any agent. Driving the CLI through a pseudo-terminal (PTY) keeps the genuine interactive TUI; reading the on-disk transcript recovers structured content without an SDK.

## What Changes

- Introduce a-thing's first packages in a ports-and-adapters layout: `@athing/sdk` (the contract), `@athing/engine` (the agent-agnostic machinery), and `@athing/adapter-claude-code` (the first concrete agent), plus `apps/server` and `apps/ui` to bootstrap and visually test the stack.
- `@athing/sdk` carries no implementation — only the ports and types: the `AgentSession` public-API contract, the `AgentDefinition` adapter contract, the `HookEvent` contract, and the canonical event model + status enum + option types. Future agents are added as new adapter packages that implement the `AgentDefinition` contract.
- `@athing/engine` is the machinery: the `AgentSession`, a `PtyTransport` (spawn the user's login shell, launch `claude`, expose raw-byte I/O and resize), a hook ingress (install lifecycle hooks, run a local HTTP receiver, call the adapter's `parseHook` to emit normalized `HookEvent`s), a status mapper (contract enum -> 5-state model, generic), a transcript reader (read the session JSONL delta via the adapter's parse function), and the glue that drives all of this from the injected `AgentDefinition` (its config data + its parse functions). The engine never imports a specific adapter; the adapter is injected at the composition root.
- `@athing/adapter-claude-code` is a hybrid `AgentDefinition`: declarative config (launch flags, hook-install spec, CLI version range) plus small functions where logic varies per agent (`parseHook` raw->`HookEvent`, `transcriptPath` incl. cwd-encoding, `parseTranscriptEntry`->content). The engine consumes its data and calls its functions.
- Transport is a per-session feature inside the single `@athing/engine`: the PTY transport is built for v1; a headless stream-json transport is a designed-for, not-built feature (a second internal code path added later, selected per session), not a separate package.
- Guarantee raw, unmangled bytes end-to-end (no ANSI stripping, no UTF-8 re-decode hops) and PTY resize propagation, so the terminal renders faithfully.
- Provide `apps/server` (Bun.serve; the composition root — injects the adapter into the engine and exposes a session over WebSocket + HTTP) and `apps/ui` (react-router SPA) as a minimal vertical slice / dev harness to validate the architecture.
- No production application UI. No API key handling. Permissions punted via `--dangerously-skip-permissions` (user answers prompts in the terminal).

## Capabilities

### New Capabilities

- `agent-session`: the engine's public session contract — lifecycle (start/send/input/interrupt/resize/kill/resume) with ready-gating, the canonical event model (data/status/content), the `HookEvent` contract (the engine's lifecycle entry point), and the raw-byte + resize guarantees.
- `pty-transport`: interactive transport built on a pseudo-terminal — spawn login shell, launch the agent command, bidirectional raw-byte I/O, terminal resize.
- `hook-ingress`: a generic lifecycle receiver — the engine installs hooks per the adapter's hook config, runs a loopback receiver, verifies the per-session token, validates the envelope, and calls the adapter's `parseHook` to emit normalized `HookEvent`s. The receiver/auth mechanism is generic engine code; the raw->contract parsing is the adapter's function.
- `agent-status`: status semantics — consume `HookEvent`s (transport-blind), map the contract event type to `{ IDLE | WORKING | WAITING_INPUT | DONE }` generically, drive the state machine idempotently.
- `agent-content`: transcript-derived structured content — on `PostToolUse`/`Stop` `HookEvent`s read the session JSONL delta and emit typed events (tool_use, edits, usage, cost) keyed by session id.
- `claude-code-agent`: the hybrid `AgentDefinition` adapter contract plus the first concrete definition for Claude Code — config data (launch flags, hook-install spec, CLI version range) and parse functions (`parseHook`, `transcriptPath`, `parseTranscriptEntry`).
- `dev-harness`: a minimal vertical slice — `apps/server` exposes a session over WebSocket + HTTP as the composition root; `apps/ui` is a simple SPA that renders the terminal, status, and content — used to bootstrap and validate the architecture, not as a product UI.

### Modified Capabilities

<!-- none — greenfield -->

## Impact

- New packages in the existing turbo workspace: `packages/sdk` (`@athing/sdk`, ports + types, zero deps), `packages/engine` (`@athing/engine`, depends on sdk), `packages/adapter-claude-code` (`@athing/adapter-claude-code`, depends on sdk). A headless stream-json transport is reserved for v2 as a feature inside `@athing/engine` (no extra package).
- New apps: `apps/server` (Bun.serve; depends on engine + adapter + sdk) and `apps/ui` (react-router SPA; depends on sdk types + talks to the server over the network).
- Dependency rule: the engine depends only on the sdk ports and never imports an adapter; the adapter is injected at the `apps/server` composition root, and apps import the engine directly.
- New runtime dependency: a pseudo-terminal (PTY) binding (native addon; runs under Bun), confined to `@athing/engine`.
- Local HTTP listener bound to `127.0.0.1` for the hook bridge.
- Writes to the user's `~/.claude/settings.json` (hook registration) and reads `~/.claude/projects/**`.
- Requires an installed, logged-in `claude` binary; the SDK never handles credentials.
- Out of scope: a production web UI, PTY daemon / cross-restart persistence, persistent session store, additional agent adapters, a programmatic permission control plane.
