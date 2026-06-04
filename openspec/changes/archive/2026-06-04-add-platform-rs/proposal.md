## Why

The native terminal-backend daemon is wire-compatible with the reference daemon, but the only
way to select it today is to hand-set an environment override pointing at a build artifact. No
host package owns building, locating, and supervising the native binary as a first-class
backend. The host layer that implements the engine platform ports is currently runtime-coupled
to one supervision/build strategy; a composition root cannot choose the native backend without
out-of-band setup.

## What Changes

- Introduce a new host package that implements the engine platform ports (daemon transport,
  file-read source, agent/daemon resolution, hook ingress, setup, and supervision) with the
  native daemon as its first-class, packaged backend.
- The new host SHALL locate the native daemon by building or discovering its compiled artifact —
  not by relying on an ambient generic-binary lookup — and SHALL supervise it (adopt a live
  instance via the shared manifest, or spawn one) over the existing wire contract.
- The host SHALL reuse the shared wire codec, framing, manifest, and snapshot encoding contracts
  unchanged; it adds no new protocol surface.
- The host SHALL account for the native backend's deliberately narrower control plane (hook
  ingress as an optional negotiated capability; no version gate; no turn-cancel/interrupt
  semantics; verbatim input) when wiring the platform ports, degrading absent capabilities
  rather than assuming them.
- A composition root SHALL be able to select this host in place of the existing one with no
  engine or protocol change.

## Capabilities

### New Capabilities

- `rust-platform-host`: A host package that implements the engine platform ports backed by the
  native terminal daemon — native-artifact build/resolution, manifest-based supervision
  (adopt-or-spawn) over the shared wire contract, file-read source, agent/daemon resolution,
  hook-ingress wiring as a negotiated capability, and setup — selectable by a composition root
  without engine or protocol changes.

### Modified Capabilities

<!-- None. The engine platform ports, wire protocol, and native daemon behavior are reused
     unchanged; this change only adds a new host implementation of existing port contracts. -->

## Impact

- New package implementing the existing `engine-platform-ports` contracts; sibling to the
  current host package, depending only on the contracts/types package.
- Consumes the native daemon's compiled artifact (build/locate step) and the shared wire,
  manifest, and snapshot contracts. No changes to the engine, the wire protocol, the native
  daemon, or the existing host package.
- A composition root gains a selectable native-backed host; selection no longer requires an
  out-of-band environment override.
