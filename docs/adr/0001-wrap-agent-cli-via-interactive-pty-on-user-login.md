# 0001. Wrap the coding-agent CLI via interactive PTY on the user's own login

- Status: accepted
- Date: 2026-06-01

## Context

a-thing must drive a coding-agent CLI (Claude Code first) from a host application so any UI can integrate it. Three integration strategies exist: an interactive pseudo-terminal (PTY) that exchanges raw bytes; a headless structured stream (`-p`/SDK) that emits typed events; or a vendor agent library. We want a genuine interactive terminal a future UI can render faithfully, generalization across CLIs, and no reimplementation of the agent loop. Auth and billing also matter: riding the user's installed, logged-in CLI avoids holding credentials, and on a subscription the interactive limits are more generous than the metered Agent SDK credit.

## Decision

Drive the agent as an interactive process inside a PTY, exchanging raw bytes, and authenticate by riding the user's existing CLI login — no API key, no vendor SDK. The SDK never handles credentials. The constraint is one subscription = one user (bring-your-own-login); any multi-user/commercial deployment must use API keys under the Commercial Terms and is out of scope here. A headless structured transport remains a possible future per-session mode (see ADR-0002).

## Consequences

- Faithful interactive TUI for terminal rendering; works for any CLI with no per-agent protocol; no credential handling.
- Output is opaque bytes — structured content is recovered separately (ADR-0006).
- Automating the interactive transport is a ToS-gray area Anthropic reserves for human use and may police; the officially-blessed `-p`/SDK path is metered to a capped credit. Acceptable for individual local use; revisit for any productized multi-user path.
- Native PTY dependency (node-pty) and platform scope macOS/Linux for v1.
