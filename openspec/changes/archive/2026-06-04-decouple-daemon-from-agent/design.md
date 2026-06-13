## Context

The daemon (`@athing/daemon`) should be a generic terminal multiplexer, but agent policy has
leaked into it. Concretely, in the current code:

- `resolve.ts` defaults the command to `claude`, searches hardcoded install locations, honors
  `CLAUDE_CODE_EXECUTABLE`, and runs `checkCliVersion` — all agent-specific.
- `pty-transport.ts:137` launches via `sh -lc "exec ${binary} ${args}"` — a login-shell
  exec-replace wrapper whose purpose is to load the user environment while hiding shell noise.
- `pty-transport.ts:14` defines `INTERRUPT_KEY = "\x1b"` and `pty-session.ts` exposes
  `interrupt()` — an agent turn-cancel.
- `pty-session.ts:118` injects `ATHING_BRIDGE_URL` (the hook socket) into the child env.

Meanwhile the adapter (`AgentDefinition`) already owns the launch template, hook-install spec,
and version range, and the engine is contractually agent-blind. So the agent policy in the
daemon is duplication that belongs upstream. The daemon already calls `installLoginShellEnv()`
at startup (`main.ts:37`), which makes the per-session exec-replace wrapper redundant for the
purpose of loading the user environment.

## Goals / Non-Goals

**Goals:**

- Make the daemon launch exactly the command it is given (default: login shell), spawned
  directly with the login-shell environment it already installs at startup.
- Generalize command resolution to: explicit path, then login-shell PATH, then login shell.
- Remove agent policy from the daemon: default binary, install locations, version gate,
  exec-replace wrapper, interrupt key.
- Relocate that policy: binary resolution + interrupt sequence into the adapter; version
  detection and interrupt-as-raw-input into the engine.
- Keep the wire contract, snapshot encoding, registry, replay, exit qualifier, and lifecycle
  byte-for-byte unchanged.

**Non-Goals:**

- The Rust port and benchmark (separate change).
- Reworking hook parsing, status mapping, or transcript handling.
- Windows support; multi-user/commercial concerns.

## Decisions

### D1: Spawn directly, drop the exec-replace wrapper

Replace `sh -lc "exec ${binary} ${args}"` with a direct spawn of the resolved command and args.
The login-shell environment is already installed process-wide at startup and inherited by
children, so the wrapper's environment-loading purpose is already covered. Spawning directly
(no shell) also means there is no shell prompt to hide — the "no shell noise" property falls out
for free instead of being engineered via exec-replace.

- _Default command_: when the launch config carries no command, spawn `$SHELL` (the user's login
  shell) so the daemon is usable as a plain terminal.

### D2: Generic resolution in the daemon

Keep a small resolver in the daemon: absolute path used as-is; bare name resolved via the
login-shell PATH; `BinaryNotFound` when a named command cannot be found. Remove the `claude`
default, the hardcoded `COMMON_LOCATIONS`, the `CLAUDE_CODE_EXECUTABLE` override, and
`checkCliVersion`. Agent-specific resolution (override path, install locations) moves into the
adapter; the adapter passes a launchable command to the daemon.

### D3: Interrupt becomes raw input driven by adapter config

Remove `INTERRUPT_KEY` and the daemon `interrupt()` command. Add an interrupt-sequence datum to
`AgentDefinition`; the `claudeCode` adapter sets it to ESC. The engine's `interrupt()` writes
those bytes through the existing raw-input path. The drive plane forwards input verbatim and
special-cases nothing.

### D4: Hook ingress stays, but agent-blind

The hook ingress socket remains owned by the daemon because it must outlive the engine host
process. It is a generic relay: it forwards raw hook payloads and never parses them (parsing is
already `adapter.parseHook` in the engine). The hook socket address is supplied to the spawned
process via the launch-config environment the caller provides (see D6), rather than injected by
daemon-internal agent plumbing — keeping the daemon's launch path free of agent specifics. The
session `token` stays a discrete spawn field because the hook ingress needs it to authenticate
per-session callbacks; that is session identity, not agent policy.

### D6: Generic launch-config environment

Today the spawn frame carries discrete agent fields (`hookSocketPath`, and the daemon injects
`ATHING_BRIDGE_URL`/`ATHING_SESSION_ID`/`ATHING_SESSION_TOKEN` by name in `pty-session.ts`),
while `pty-transport.ts` curates a base allowlist (PATH, HOME, USER, LOGNAME, SHELL, LANG, TERM,
COLORTERM, SSH*AUTH_SOCK) from its startup-installed `process.env`. The daemon naming the
`ATHING*\*` vars is the env-layer leak.

Replace the discrete agent env fields with a single generic `env: Record<string,string>` on the
spawn frame. The daemon computes the child environment as: its generic terminal base allowlist
(unchanged — PATH/HOME/TERM/COLORTERM/etc. derived from the login-shell env it installs at
startup) merged with the caller-supplied `env`, caller entries winning. The engine/adapter put
`ATHING_BRIDGE_URL` (the deterministic `~/.athing/hooks.sock`), `ATHING_SESSION_ID`, and
`ATHING_SESSION_TOKEN` into `env`; the daemon references none of those names.

- _Kept on the daemon_: the terminal base env (TERM/COLORTERM/PATH/HOME/...) is a generic
  terminal-backend concern, not agent policy, so it stays.
- _Folded away_: the `flags` spawn field merges into `args` (flags are an agent-shaped split the
  adapter performs before sending). `token` and `sessionId` stay discrete for the registry and
  hook authentication.
- _Alternative_: pass the whole environment from the caller and remove the daemon base allowlist.
  Rejected: the base TERM/PATH/etc. is what makes the daemon a usable terminal on its own, and
  centralizing it avoids every caller re-deriving login-shell env.

### D5: Version detection moves to the engine

The engine performs version detection from adapter config and emits `VersionUnsupported`. The
daemon performs no version gate. This matches the existing `claude-code-agent` requirement that
the engine, not the daemon, owns version awareness.

## Risks / Trade-offs

- **Breaking the launch path mid-stack** -> the engine/adapter must now resolve and pass a
  launchable command and the hook env. _Mitigation_: land adapter resolution + engine wiring in
  the same change; cover with the existing `pty-transport` and `claude-code-agent` scenarios.
- **Losing the "no shell noise" guarantee** -> if a command is somehow launched via a shell, a
  prompt could leak. _Mitigation_: direct spawn (no shell) makes this structurally impossible;
  assert "first bytes are the command's output" in tests.
- **Env parity regressions** -> children must still see the full login environment.
  _Mitigation_: `installLoginShellEnv()` already runs at startup and is inherited; verify PATH
  and friends on spawned children.
- **Interrupt timing** -> writing ESC as raw input must reach the agent as before.
  _Mitigation_: route through the same raw-input channel the daemon already exposes; behavior
  is unchanged from the agent's perspective.

## Migration Plan

1. Add the interrupt-sequence datum to `AgentDefinition`; set it in `claudeCode`.
2. Replace the discrete spawn env fields with a generic `env` map; have the engine resolve the
   agent binary (override + locations) in the adapter and pass a launchable command plus the
   `ATHING_*` entries in `env`; fold `flags` into `args`.
3. Move version detection into the engine; drop `checkCliVersion` from the daemon.
4. In the daemon: replace the exec-replace launch with a direct spawn (default `$SHELL`),
   generalize the resolver, and remove the interrupt command and `INTERRUPT_KEY`.
5. Run the `pty-transport`, `claude-code-agent`, and `agent-session` scenarios green.

## Open Questions

- Should the daemon still expose a convenience that injects the hook-socket env when the launch
  config omits it, or is supplying it always the caller's responsibility?
- Does any current caller rely on the daemon interrupt command directly (outside the engine
  `interrupt()` path) and need updating in the same change?
