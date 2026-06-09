# 0001. Wrap the coding-agent CLI via interactive PTY on the user's own login

- Status: accepted
- Date: 2026-06-01

## Context

tillerd must drive a coding-agent CLI from a host application so any UI can integrate it. Three integration strategies exist: an interactive pseudo-terminal (PTY) that exchanges raw bytes; a headless structured stream (`-p`/SDK) that emits typed events; or a vendor agent library. We want a genuine interactive terminal a future UI can render faithfully, generalization across CLIs, and no reimplementation of the agent loop. Auth also matters: riding the user's installed, logged-in CLI avoids holding credentials.

## Decision

Drive the agent as an interactive process inside a PTY, exchanging raw bytes, and authenticate by riding the user's existing CLI login — no API key, no vendor SDK. The SDK never handles credentials. The constraint is one subscription = one user (bring-your-own-login); multi-user/commercial deployment is out of scope here. A headless structured transport remains a possible future per-session mode (see ADR-0002).

## Consequences

- Faithful interactive TUI for terminal rendering; works for any CLI with no per-agent protocol; no credential handling.
- Output is opaque bytes — structured content is recovered separately (ADR-0006).
- Interactive automation suits individual local use; the headless (`-p`/SDK) path has different usage/billing trade-offs.
- Native PTY dependency (node-pty) and platform scope macOS/Linux for v1.
