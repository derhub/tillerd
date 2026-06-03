## Why

Importing the agent adapter pulls host filesystem primitives at module load: its hook installer
reads and writes the agent settings file directly, and its transcript-path resolver reads the
home directory from an ambient host global. A renderer-side host (the desktop web view) cannot
import the adapter at all, so it cannot hand the adapter to the engine to drive a live session.
This is the prerequisite that unblocks the live desktop integration.

## What Changes

- **BREAKING**: Remove the imperative `installHooks(notifyCommand, logger)` and
  `uninstallHooks(logger)` methods from the `AgentDefinition` contract. The adapter no longer
  performs filesystem I/O.
- The adapter supplies hook installation as **declarative spec data** (settings-file location,
  command-template, event list, hook marker, matcher rule) plus **pure planning functions** that,
  given the current settings and a notify command, return the next settings value and what
  changed. The adapter computes the mutation; it never reads or writes a file.
- **BREAKING**: `transcriptPath(sessionId, cwd)` gains an explicit agent-home input
  (`transcriptPath(sessionId, cwd, agentHome)`); it no longer reads the home directory from an
  ambient host global, and its path assembly uses pure string operations.
- **BREAKING**: Remove the imperative `resolveCommand()` method (it imported `node:child_process`/
  `node:fs` and read `process.env` at module load). The adapter instead supplies a declarative
  `binaryResolution` policy (override env var, binary name, common install locations); the host
  performs the lookup and passes the resolved command to the engine as a startup value.
- The host owns all filesystem work: it reads, backs up, and writes the settings file using the
  adapter's declarative spec and pure plan, and it resolves the agent-home value at startup.
- The engine receives the resolved agent-home as a startup value and passes it through to the
  adapter's transcript-path resolution.
- Net effect: the adapter module is import-safe in any runtime (zero host primitives at module
  load), so a renderer host can import it and drive `engine.start(adapter)`.

## Capabilities

### New Capabilities

<!-- none — this refactors existing behavior -->

### Modified Capabilities

- `claude-code-agent`: the `AgentDefinition` contract drops the imperative hook-install/uninstall
  methods in favor of declarative hook-install spec data plus pure planning functions; the adapter
  is import-safe with zero host-primitive access at module load; `transcriptPath` takes an explicit
  agent-home input.
- `engine-platform-ports`: the host-supplied startup-resolved values include the agent-home value,
  which the engine passes through to transcript-path resolution; the engine reads no ambient home
  or path global.

## Impact

- `@athing/sdk` — `AgentDefinition` contract shape (remove install/uninstall methods, add
  hook-install spec data, change `transcriptPath` signature).
- `@athing/adapter-claude-code` — `hook-installer.ts` becomes pure planning; `transcript-path.ts`
  drops the home/path host primitives; `resolve.ts` is removed in favor of pure
  `binary-resolution.ts` data; `index.ts` exposes the declarative spec and pure functions. The
  module is fully host-primitive-free.
- `@athing/engine` — content resolution threads the agent-home startup value into `transcriptPath`;
  the spawn path uses the caller-resolved command (`EngineDeps.resolvedCommand`) instead of
  `adapter.resolveCommand()`.
- `@athing/platform-bun` — owns the settings read/backup/write orchestration over the adapter's
  spec and plan, the agent-home resolution, and the binary lookup (`resolveAgentCommand`).
- `apps/server` — composition root calls the host-side install path instead of
  `adapter.installHooks`, and resolves the agent command via the host before engine bootstrap.
- Tests for hook-installer, transcript-path, engine content, and adapter index.
