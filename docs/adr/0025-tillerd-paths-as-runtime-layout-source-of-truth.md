# 0025. A `tillerd-paths` crate is the single source of truth for the runtime layout and `TILLERD_*` surface

- Status: proposed
- Date: 2026-06-12

## Context

Where tillerd puts its runtime files — the daemon socket, gate socket, daemon manifest, and product
store — and how it finds its service binaries is resolved independently in many places. Four crates
define a runtime-directory resolver (`desktop`, `mcp-gateway`, `process-launch`, `service-host`),
several rebuild the same socket/manifest paths, the daemon/gate/notify binary resolver with its
`target/release` discovery fallback exists only in the desktop host, and the `TILLERD_*` environment
variable names are string literals across more than a dozen files.

These copies drift. A fix to one resolver does not reach the others: the desktop binary resolver
lacked the cargo-output fallback while the build placed the binary elsewhere, which repeatedly broke
`dev` and `test` until patched in one place — and only there. There is no single answer to "where does
tillerd put things, and what are its environment variables."

The runtime layout is a shared assumption of ADR-0019/service-host (runtime dir + manifest), ADR-0023
(one product store under the runtime dir), ADR-0008/0016 (daemon PTY socket), and ADR-0018 (gate
single socket). It deserves one owner.

## Decision

Introduce a leaf-level `tillerd-paths` crate as the single source of truth, depended on inward by
every service and the host. It owns:

- **Runtime-directory resolution** — `TILLERD_DIR` then `~/.tillerd` — as the only implementation,
  plus an override-aware form for the service host's CLI override.
- **Runtime-layout path builders** — daemon socket, gate socket, daemon manifest, product store —
  as pure functions of a directory; the file names are defined only here.
- **Service-binary resolution** — daemon, gate, notify — by one precedence: the binary's override
  environment variable (when it names an existing file), then `bin/<name>` or
  `target/{release,debug}/<name>` under the working directory or an ancestor, then `~/.local/bin/<name>`.
- **The `TILLERD_*` environment-variable name constants** it governs (the runtime directory and the
  binary overrides); auth-token value variables are out of scope.

The crate depends only on the standard library and a home-directory helper, so it cannot create
dependency cycles. Every prior owner migrates to it and deletes its local resolver, builders, and
hardcoded names. This is a leaf foundation, not a module of any one crate: making it a module would
invert the dependency graph, so a dedicated crate is the correct shape despite the project's
prefer-modules default (and it was explicitly requested).

This ADR supersedes no prior ADR. It changes no path, file name, or environment semantics; it
records where they now live and supersedes the scattered-resolver pattern.

## Consequences

- **Easier:** one place to change the runtime layout or env surface; resolution is identical across
  every consumer — notably the `target/{release,debug}` binary fallback now applies everywhere, which
  removes the missing-fallback class of dev/test breakage.
- **Harder / costs:** one more workspace crate, and a wide migration (seven consumers) whose blast
  radius reaches boot — mitigated by per-consumer steps gated on workspace `cargo test` + clippy.
- **Neutral:** non-desktop consumers gain the binary discovery fallback they lacked (the override env
  still wins first); the TS-side `TILLERD_*` readers are unchanged and a future mirrored constants
  module is left open.
