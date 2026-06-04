## Context

The engine consumes its environment only through injected platform ports (`engine-platform-ports`):
a connected daemon transport, a file-read source, a logger, the working directory, the resolved
agent invocation, the agent-home location, and the hook callback configuration. The existing host
package implements those ports against the reference daemon, resolving a generic daemon-binary name
and supervising it over a Unix-socket control plane with a shared length-prefixed frame codec,
manifest, and snapshot encoding.

A native terminal daemon already exists and is wire-compatible: same sockets, manifest, framing, and
snapshot cell encoding. Today the only way to use it is to point the existing host's generic binary
override at the native build artifact. It deliberately implements a narrower control plane than the
agent-oriented reference: hook ingress is an optional negotiated capability, and there is no CLI
version gate, no turn-cancel/interrupt semantics, and no live upgrade handoff (an upgrade request
degrades).

This change adds a dedicated host that makes the native daemon a first-class, packaged backend:
owning its artifact build/resolution and supervision, and wiring the platform ports around its
narrower control plane — selectable by a composition root with no engine or protocol change.

## Goals / Non-Goals

**Goals:**

- A host package implementing the existing platform-port contracts with the native daemon as backend.
- First-class artifact resolution: override, then build-output location, then install locations — not
  a bare generic-name lookup.
- Manifest-based adopt-or-spawn supervision over the shared wire contract, with a bounded startup
  deadline and typed failures.
- Wire only negotiated capabilities; degrade features the native daemon does not implement.
- Reuse the shared codec, framing, manifest, and snapshot contracts unchanged — no new protocol.
- Selectable in place of the reference host with no engine change.

**Non-Goals:**

- No changes to the engine, the wire protocol, or the native daemon.
- No new wire messages, snapshot encoding, or manifest fields.
- No CLI version gate, turn-cancel/interrupt, or live upgrade handoff (the native daemon omits these).
- No new validation surface beyond the shared contracts.

## Decisions

**Thin host that reuses the reference host, specializing only daemon-binary resolution.** Inspection of
the reference host showed that — because the wire codec, framing, manifest, socket paths, and snapshot
encoding are shared contracts, and the agent-CLI version gate, interrupt, and transcript/setup paths are
backend-independent — the only behavior that genuinely differs for the native backend is *which daemon
binary is resolved and spawned*. The native host therefore depends on the reference host and re-exports
its platform-port surface unchanged, overriding only the supervision entry point to supply a native
binary resolver. Alternatives rejected: a full sibling duplicate (≈95% identical files — duplication the
contracts cannot remove); a runtime backend *flag* on the reference host (interleaves two backends in one
call site). Injecting a resolver is narrower than a flag — one function, no branching control plane.

**Reference host takes an optional daemon-binary resolver.** The reference host's adopt-or-spawn keeps its
default resolver (no change for existing callers) but accepts an optional resolver override. The native
host passes its own. The shared supervision body — adopt-live, clear-stale-socket, spawn-detached,
deadline-bounded connect — is reused verbatim, preserving the "identical observable behavior across the
injection seam" requirement.

**Native artifact resolution order: override → build-output → install locations.** The native artifact is
a compiled build output at a known path, not something expected on the ambient PATH under a generic name.
The native resolver checks an explicit override first (operability/escape hatch), then the native build-
output location, then established install locations, and raises a typed not-found error naming the
override and the build step. This avoids the failure mode where a generic-name lookup silently selects the
reference daemon instead.

**Narrower native control plane handled by graceful degradation, not explicit capability-gating code.**
The native daemon's narrower control plane (optional hook ingress, no version gate, no turn-cancel, no
upgrade handoff) needs no host-side gating: a control message the daemon does not implement is a no-op,
and if the daemon does not run a hook-ingress listener the host's hook-socket configuration simply yields
no hook frames. This is the ADR-0007 plane-degradation behavior achieved through the shared wire contract
rather than backend-specific branching. The host adds no new protocol surface.

## Risks / Trade-offs

- [Coupling: the native host depends on the reference host package] → Mitigation: the dependency is exactly
  what removes duplication — the native host carries only a resolver and a thin override. The seam is the
  optional resolver argument, a stable, minimal surface. If the reference host is later renamed or its
  Bun-specific bits split out, the native host follows that single dependency.
- [Capability drift: the native daemon's negotiated set changes over time] → Mitigation: gate every
  optional feature on the negotiated capability and degrade on absence, so an unadvertised feature is a
  no-op rather than a crash.
- [Artifact resolution selecting a stale or wrong build] → Mitigation: explicit override takes precedence;
  the typed not-found error names the build step so a missing artifact is actionable rather than silently
  falling back to the reference daemon.
- [Behavioral divergence from the reference host across the injection seam] → Mitigation: the spec requires
  identical session-event ordering; verify with a substitute-transport parity test reusing the existing
  engine port tests.

## Migration Plan

Additive — no migration. The package is new and opt-in; a composition root selects it explicitly. Rollback
is removing the selection (the reference host is untouched). No protocol, manifest, or engine version change,
so a native-backed and reference-backed deployment interoperate over the same sockets and manifest.

## Open Questions

- Should artifact build be triggered by the host at resolution time, or assumed pre-built with the host only
  discovering the output? Leaning discover-only (build is a separate explicit step, mirroring the native
  crate's out-of-workspace build), with the typed error pointing at the build command.
- Does the composition root need to select the backend at runtime (config/env) or is compile-time wiring
  sufficient for v1? Compile-time selection is assumed sufficient until a runtime switch is required.
