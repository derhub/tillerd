## 1. Adapter and SDK: own the agent policy

- [x] 1.1 Add an interrupt-sequence datum to the `AgentDefinition` contract in `@athing/sdk`
- [x] 1.2 Set the interrupt sequence to ESC in the `claudeCode` adapter
- [x] 1.3 Move agent binary resolution (explicit override path, then common install locations) into the `claudeCode` adapter, exposing a launchable command to the engine
- [x] 1.4 Update adapter tests to cover resolution and the interrupt-sequence datum

## 2. Spawn protocol: generic env map

- [x] 2.1 Replace the spawn message's discrete app-named fields with a generic `env: Record<string,string>` map; fold `flags` into `args`; keep `token` and `sessionId` discrete
- [x] 2.2 In the daemon, compute child env as the generic terminal base allowlist merged with the caller's `env` (caller wins); stop injecting `ATHING_*` by name in `pty-session.ts`
- [x] 2.3 Update the wire-message schema and any validators for the new spawn shape

## 3. Engine: drive resolution, version, interrupt, env

- [x] 3.1 Resolve the agent binary via the adapter and pass a launchable command in the daemon launch config
- [x] 3.2 Perform agent version detection in the engine from adapter config and emit `VersionUnsupported` on mismatch
- [x] 3.3 Build the launch-config `env` map in the engine, including `ATHING_BRIDGE_URL` (deterministic hook socket), `ATHING_SESSION_ID`, and `ATHING_SESSION_TOKEN`
- [x] 3.4 Implement `interrupt()` by writing the adapter's interrupt sequence through the raw-input path; remove use of any daemon interrupt command

## 4. Daemon: generic launch

- [x] 4.1 Replace the `sh -lc "exec ..."` launch with a direct spawn of the resolved command and args, inheriting the startup-installed login-shell environment
- [x] 4.2 Default to the user's login shell when the launch config supplies no command
- [x] 4.3 Assert no wrapper-shell prompt/echo precedes the spawned process's output

## 5. Daemon: generic resolution

- [x] 5.1 Generalize `resolve.ts`: absolute path as-is, bare name via login-shell PATH, `BinaryNotFound` on miss
- [x] 5.2 Remove the `claude` default, `COMMON_LOCATIONS`, `CLAUDE_CODE_EXECUTABLE` override, and `checkCliVersion`
- [x] 5.3 Update daemon resolution tests for the generic behavior

## 6. Daemon: remove interrupt special-casing

- [x] 6.1 Remove `INTERRUPT_KEY` and the daemon `interrupt()` command; forward input bytes verbatim
- [x] 6.2 Remove the interrupt command from the IPC protocol/messages and its handlers

## 7. Hook ingress stays agent-blind

- [x] 7.1 Confirm the daemon hook ingress only relays raw payloads (no parsing) and keep it owned by the daemon for restart durability
- [x] 7.2 Verify the hook-socket env reaches the spawned process via the launch config rather than daemon-internal agent plumbing

## 8. Verification

- [x] 8.1 Run the modified `pty-transport` scenarios (generic launch, generic resolution, interrupt-key removed) green
- [x] 8.2 Run the modified `claude-code-agent` scenarios (adapter resolution, interrupt-sequence, engine version detection) green
- [x] 8.3 Run the modified `agent-session` interrupt-versus-kill scenario green
- [x] 8.4 Confirm wire framing, snapshot encoding, registry, replay, exit qualifier, and lifecycle behavior are unchanged
