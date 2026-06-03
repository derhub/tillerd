## Context

The adapter currently performs host I/O at two seams. Its hook installer imports filesystem and
home-directory primitives at module load and reads/backs-up/writes the agent settings file
directly. Its transcript-path resolver reads the home directory from an ambient host global. As a
result, importing the adapter module fails in a renderer-class host (a desktop web view), which
blocks the renderer from handing the adapter to the engine to drive a live session.

The codebase already follows ports-and-adapters: the engine receives injected contracts
(transport, file-read, logger) and startup-resolved values from its caller; host-specific I/O
lives in the platform host package; the adapter is meant to be declarative config plus pure
parse functions. This change closes the two remaining seams where the adapter still reaches host
primitives.

The contract spec already describes the intended shape (declarative hook-install spec + parse
functions, no imperative install method); the implementation had drifted by adding imperative
`installHooks`/`uninstallHooks` methods. This change realigns code to that intent and extends it
with pure planning functions and an explicit agent-home input.

## Goals / Non-Goals

**Goals:**

- The adapter module is import-safe in any runtime: zero host-primitive access at module load and
  within every contract function.
- Hook installation/removal expressed as a pure transform over the current settings value; the
  host owns the read/backup/write.
- `transcriptPath` takes an explicit agent-home input; the engine threads a caller-resolved
  agent-home value through to it.
- No change to observable session behavior or to the on-disk settings/transcript layout.

**Non-Goals:**

- The desktop renderer's actual wiring of `engine.start(adapter)` (that is the downstream change
  this unblocks).
- Any change to the loopback hook ingress, the notify script, or the hook payload contract.
- Supporting a settings location other than the agent's existing one, or a configurable
  agent-home beyond resolving it once at startup.

## Decisions

### Decision: Pure planning functions, host owns the file I/O

Replace the imperative `installHooks`/`uninstallHooks` adapter methods with pure functions
`planHookInstall(currentSettings, notifyCommand)` and `planHookUninstall(currentSettings)` that
return `{ settings, changed, events }`. The host (platform host package) reads the settings file,
calls the planning function, and — only when `changed` — backs up and atomically writes the
result. The "already installed / already absent → no change" idempotency moves into the pure
plan, where it is trivially testable without a filesystem.

- **Why over injecting a settings-store port into the adapter:** a port would keep an I/O
  orchestration responsibility inside the adapter and still require the adapter to know about
  read/backup/write ordering. A pure transform is simpler, fully unit-testable with plain
  objects, and leaves all I/O concerns in the one layer that already owns them.

### Decision: `transcriptPath` gains an explicit `agentHome` parameter

Change the signature to `transcriptPath(sessionId, cwd, agentHome)` and build the path with pure
string operations (the existing cwd-encoding rule plus `/`-joining), removing the home-directory
and path host primitives. The engine supplies `agentHome` from a caller-resolved startup value
when it calls the adapter during transcript content resolution.

- **Why over a global/config singleton:** the engine-platform-ports contract already establishes
  "host supplies startup-resolved values"; agent-home is one more such value, resolved once by the
  host and passed inward. Threading it as a parameter keeps the adapter function pure.

### Decision: Host I/O lives in the platform host package; the composition root calls it

Move the settings read/backup/atomic-write orchestration into the platform host package alongside
the other host concerns (file-read contract impl, binary/version resolution, notify-script
preparation). The composition root calls that host-side `installHooks` instead of
`adapter.installHooks`. Agent-home resolution also lives in the host and is passed to the engine.

- **Why:** keeps the adapter free of host primitives and concentrates runtime-specific filesystem
  code in the single package designated for it, matching the existing arrangement.

### Decision: Binary resolution becomes adapter policy data + host lookup

The imperative `resolveCommand()` method imported `node:child_process`/`node:fs` and read
`process.env` at module load, so importing the adapter still failed in a renderer-class host — the
same seam class as the hook installer. Replace it with declarative `binaryResolution` data on the
adapter (override env var, binary name, common install locations) plus a host-side
`resolveAgentCommand(spec)` in the platform host package that performs the lookup (override path,
then login-shell PATH, then common locations). The host resolves the command once at startup and
passes it to the engine as a startup value; the engine spawns with it directly instead of calling
`adapter.resolveCommand()`.

- **Why:** this is the third host seam in the adapter and the only one left after the hook and
  transcript work. Closing it makes the adapter module fully import-safe (zero host primitives at
  load and in every contract function), and it aligns the code with the existing
  engine-platform-ports requirement that the host supply the resolved agent invocation and the
  engine perform no executable resolution — which `proxy.ts` previously violated by calling
  `adapter.resolveCommand()` inside the engine.

### Decision: Remove the methods from the contract (breaking, pre-v1)

Drop `installHooks`/`uninstallHooks` from `AgentDefinition` outright rather than keeping
deprecated shims. Pre-v1 the project breaks contracts freely. Test doubles of `AgentDefinition`
in the engine tests are updated in the same change.

## Risks / Trade-offs

- **Settings-file write correctness moves to the host** → the host-side install path keeps the
  existing atomic write (temp file + rename) and timestamped backup; golden tests on the pure plan
  cover the JSON mutation, and a host-side test covers read/backup/write ordering.
- **Agent-home threading touches the engine content path** → covered by the behavior-preservation
  requirement: engine content tests assert identical transcript-path resolution given the
  resolved agent-home, using a substitute value.
- **Breaking contract change ripples to all `AgentDefinition` constructions** → the call sites are
  few (composition root plus a handful of test doubles); each is updated in this change.

## Migration Plan

Single-repo, pre-v1, no runtime migration. Update the contract, adapter, engine threading, host
package, composition root, and affected tests together so the build stays green. No data or
on-disk format changes; existing settings files and transcript paths are unaffected.

## Open Questions

- None blocking. Resolution of the agent-home value reuses the host's existing home-directory
  access; if a future host needs a non-default agent-home, the startup-value seam already
  accommodates it.
