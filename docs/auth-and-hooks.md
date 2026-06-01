# Auth, billing, and hook management

## Auth / billing constraint

This SDK drives the CLI through the user's own installed, logged-in session — **no API key**.

**One subscription = one user (bring-your-own-login).** Individual local use is fine.
Running a multi-user service on a single subscription violates Anthropic's Terms of Service
(no third-party login sharing, no account sharing). Multi-user deployments require API keys
under Anthropic's Commercial Terms.

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

## Hook uninstall

To remove the SDK's hooks from the agent's settings:

```ts
import { uninstallHooks } from "@athing/engine";
import { claudeCode } from "@athing/adapter-claude-code";

uninstallHooks(claudeCode.hookInstall, logger);
```

This removes only the SDK's entries; existing user hooks are left intact.
