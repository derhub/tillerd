# 0018. Gate presents a single route-multiplexed socket; MCP is socket-only

- Status: accepted
- Date: 2026-06-09

## Context

ADR-0016 made the gate the single trust boundary for agent-facing traffic. In practice the gate grew one Unix socket per face — hook, tool, subscribe, admin — plus an MCP face whose primary transport was loopback HTTP on an ephemeral TCP port published to a `.url` sidecar. Four of the five faces already share the length-prefix frame codec (ADR-0009), and the MCP socket face already opens with a one-frame admission handshake before upgrading to the MCP protocol. The multi-socket split is therefore incidental: every gate-native producer and consumer must know a distinct well-known filename, and the lone TCP port forces a non-derivable address and a stale-prone sidecar.

The MCP HTTP transport, though documented as the primary client-facing one, has no consumer in the codebase: nothing reads its discovery file, no MCP server is registered for the agent, and the agent adapter wires no MCP. Removing it while unused is free. (This concerns the gate's own MCP face only; the standalone MCP gateway daemon of ADR-0014 keeps its standard HTTP front for external clients and is unaffected.)

A route preamble is a small generalization of the admission handshake the MCP face already performs, so one socket can carry every face without losing any face's behavior.

## Decision

The gate SHALL present exactly one named Unix domain socket at a path derived from the runtime directory (`$ATHING_DIR/gate.sock`). Every connection SHALL open with one length-prefixed route preamble — `{ route, session, token?, wireVersion }` — encoded with the shared frame codec; the gate demultiplexes on `route` (`Hook`, `Tool`, `Subscribe`, `Admin`, `Mcp`) to the existing per-face behavior. Each route keeps its post-admission lifecycle; the `Mcp` route, after verifying the preamble token, upgrades the connection to the MCP protocol over a maintained protocol library and stops framing.

A single centralized policy maps each route to its required credential: `Hook`/`Tool`/`Mcp` require a valid per-session token; `Admin` requires the admin token, distinct from any session token; `Subscribe` requires none. The privileged registry-mutate face (`Admin`) is thus separated by credential rather than by socket file — trading a physical wall for a single front door, on the condition that the route-to-credential mapping lives in one place and is guarded by a test asserting a session token cannot reach `Admin`.

The gate's MCP face becomes socket-only: the loopback-HTTP transport, its configuration selectors, and its published endpoint are removed. With the hook face already on a socket, the gate then binds no TCP listener at all and publishes no address file — every surface is a Unix socket at a derivable path.

Gate-native client peers share a client library (connect, write preamble, frame codec) and the preamble contract, but remain separate processes because their lifecycles differ irreconcilably (a hook producer is spawned per event and exits; an MCP bridge is spawned once and pumps full-duplex for a session; a subscribe consumer holds a long-lived read stream).

## Consequences

- One trust boundary becomes one socket: a single bind, a single accept loop, a single discovery path, and one place to add cross-cutting admission policy. ADR-0016's intent is realized literally.
- The gate is fully Unix-socket and port-free: no ephemeral TCP port, no loopback-origin check, no published address files; every path derives from `$ATHING_DIR`, so a host restart needs no re-resolution.
- The wire envelope (route, session, token) converges with the gate's internal inbound shape, eliminating the per-face envelope drift the codec consolidation began removing.
- The `Admin` face loses its physical-socket isolation; its protection now rests entirely on the centralized credential policy, which becomes a required, tested invariant. `Subscribe` remaining credential-free is now an explicit, audited choice on a shared socket.
- Adding a future face is adding a `route` value and a credential rule, not a new socket and a new discovery artifact.
- The agent cannot speak a Unix-socket MCP transport directly; when it must reach the gate's own MCP tools, a stdio↔socket bridge process (peer to the hook producer) will give it a standard front backed by the socket — deferred until such a consumer exists.
- Pre-v1 and breaking: producer, consumers, and gate re-wire to the preamble in lockstep within one change; rollback is reverting that change.
