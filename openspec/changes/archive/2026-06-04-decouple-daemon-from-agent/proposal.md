## Why

The daemon is supposed to be a generic terminal backend — own a pseudo-terminal, launch what
it is told, stream raw bytes, snapshot, and survive restarts — but it currently bakes in
agent-specific policy: it defaults the launch command to a specific agent binary, searches
hardcoded install locations, gates on that agent's CLI version, launches via a login-shell
`exec`-replace to hide shell noise, and special-cases the ESC key as a turn-cancel. That policy
already has a proper home — the adapter (`AgentDefinition`) — which the engine is meant to keep
agent-blind. Decoupling makes the daemon a clean, reusable terminal multiplexer and is a
prerequisite for a second (native) daemon implementation that should not have to re-learn any
agent semantics.

## What Changes

- **BREAKING** The daemon launches the command it is given, verbatim: a launch config carries
  the command, arguments, working directory, and environment, and the daemon spawns it directly
  (no login-shell `exec`-replace wrapper). When no command is supplied the daemon launches the
  user's login shell. The daemon already installs the login-shell environment at startup, so
  spawned commands inherit a user-terminal environment without a per-session shell wrapper.
- **BREAKING** Command resolution in the daemon becomes generic: an explicit path is used as
  given; a bare name is resolved via the login-shell PATH; otherwise the login shell is used.
  The daemon drops the agent-specific default command, the hardcoded install locations, and the
  CLI version gate.
- **BREAKING** The daemon stops special-casing the interrupt key. Input bytes are forwarded
  verbatim; cancelling a turn becomes the engine writing the adapter-defined interrupt sequence
  through the normal raw-input path. The daemon's dedicated interrupt command is removed.
- **BREAKING** The spawn frame replaces its discrete application-named fields with a generic
  `env: Record<string,string>` map (and folds `flags` into `args`). The daemon merges its generic
  terminal base environment (PATH/HOME/TERM/COLORTERM/... from the login-shell env it installs at
  startup) with the caller's map, caller winning, and references no application variable by name.
  The engine/adapter place `ATHING_BRIDGE_URL`, `ATHING_SESSION_ID`, and `ATHING_SESSION_TOKEN` in
  that map; `token` and `sessionId` remain discrete fields for the registry and hook authentication.
- Agent-specific resolution (override path, common install locations) and version detection move
  to the engine/adapter, which resolves the agent binary and passes a launchable command to the
  daemon. The adapter's `AgentDefinition` gains an interrupt-sequence datum.
- Hook ingress stays in the daemon (it must outlive the engine host process) but is framed as a
  generic, agent-blind relay: it forwards raw hook payloads and never parses them. The hook
  socket address is supplied to the spawned process via the launch-config environment rather
  than injected by daemon-internal agent plumbing.
- No change to the wire framing, the snapshot cell encoding, the session registry, the replay
  buffer, exit-qualifier translation, or the lifecycle/upgrade behavior.

## Capabilities

### Modified Capabilities

- `pty-transport`: launch becomes generic (spawn the given command directly, default login
  shell, no `exec`-replace); binary resolution becomes generic (explicit path, then login-shell
  PATH, then default login shell; no agent default/locations); the interrupt-key requirement is
  removed from the drive plane.
- `claude-code-agent`: the adapter owns agent binary resolution (override + common locations)
  and supplies a launchable command plus an interrupt-sequence datum to the engine; the engine
  performs version detection from adapter config. The daemon is no longer the source of any of
  these.
- `agent-session`: `interrupt` cancels the current turn by writing the adapter's interrupt
  sequence through the raw-input path rather than via a daemon interrupt command.

### New Capabilities

<!-- None. This change relocates existing behavior; it does not introduce a new capability. -->

## Impact

- **Touched code**: `@athing/daemon` (`resolve.ts`, `pty-transport.ts`, `pty-session.ts`,
  `server.ts`, `shell-env.ts`) — remove agent defaults/locations/version gate, drop the
  `exec`-replace launch wrapper and the interrupt command, generalize resolution; `@athing/sdk`
  `AgentDefinition` — add an interrupt-sequence datum; `@athing/adapter-claude-code` — own agent
  binary resolution and supply the interrupt sequence; `@athing/engine` — resolve via adapter,
  perform version detection, write the interrupt sequence as raw input, set the hook socket env
  on the launch config.
- **Unchanged**: wire framing, snapshot encoding, registry, replay buffer, exit qualifier,
  graceful shutdown, upgrade handoff, all socket/manifest paths.
- **Enables**: a clean generic daemon contract for the `rust-pty-daemon` change to target.
- **Out of scope**: the Rust port and the benchmark (separate change), Windows support,
  multi-user/commercial concerns.
