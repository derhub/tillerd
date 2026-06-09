# Auth, billing, and hook management

## Auth / billing constraint

This SDK drives the CLI through the user's own installed, logged-in session — **no API key**.

**One subscription = one user (bring-your-own-login).** Individual local use is fine.
Multi-user deployments are out of scope for v1.

## Hook installation

On first `engine.start()` the engine writes a notify command into the agent's settings file
(e.g. `~/.claude/settings.json`) for each configured hook event. This is a **non-destructive
merge** — existing user hooks are preserved.

Each session scopes itself via env vars injected into the PTY at launch:

- `ATHING_BRIDGE_URL` — the loopback receiver URL
- `ATHING_SESSION_ID` — the session UUID
- `ATHING_SESSION_TOKEN` — per-session secret for authenticating callbacks

Only one hook command is installed (not one per session), so concurrent sessions share the
static hook entry and are differentiated entirely by env vars.

### Runtime-free notify client

The installed hook command is a committed standalone shell script, `bin/athing-notify`
(`#!/usr/bin/env bash`), not a runtime-executed script. It reads the lifecycle payload from
stdin and `curl`s it to `ATHING_BRIDGE_URL` — over a unix socket when the value begins with `/`,
otherwise as a URL — carrying the session id and token as headers. It is fire-and-forget: bounded
runtime (`--max-time`), all output and errors suppressed, always `exit 0`, and an exit-early when
`ATHING_BRIDGE_URL` is unset.

This relies only on `bash` and `curl` being present on the target platform (macOS/Linux for v1),
so lifecycle callbacks fire even when no language runtime is resolvable on the agent's PATH. There
is intentionally **no fallback runtime**.

## Hook uninstall

To remove the SDK's hooks from the agent's settings, invoke the adapter's setup
`uninstall` with a host-provided `SetupContext`:

```ts
import { setup } from "@athing/adapter-claude-code";
import { buildSetupContext } from "@athing/platform-bun";

await setup.uninstall(buildSetupContext(notifyCommand, logger));
```

This removes only the SDK's entries; existing user hooks are left intact.
