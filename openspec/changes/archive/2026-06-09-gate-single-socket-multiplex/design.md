## Context

The gate is the single trust boundary for agent-facing traffic (ADR-0016). It reached that role incrementally: hook ingress moved off the daemon onto the gate, the frame codec was consolidated (ADR-0009), and the hook face was migrated from loopback HTTP to a framed Unix socket. The result today is five faces, each on its own well-known file:

```
  ~/.athing/gate-hook.sock        Hook       fire-and-forget   session token (envelope)
  ~/.athing/gate-tool.sock        Tool       req/resp          session token (envelope)
  ~/.athing/gate-subscribe.sock   Subscribe  ready → stream    none
  ~/.athing/gate-admin.sock       Admin      req/resp          admin token (≠ session)
  ~/.athing/gate-mcp.{sock,url}   Mcp        upgrade → MCP      session token
```

Four of the five already speak the same length-prefix codec. The MCP **socket** path already opens with a one-frame admission handshake (`{session, token}`) and then upgrades the stream to the MCP protocol — so a route selector is a small generalization of something the gate already does. The MCP **HTTP** transport, documented "primary, client-facing," has no consumer anywhere in the repository: nothing reads `gate-mcp.url`/`.sock`, no MCP server is registered for the agent, and the adapter wires no MCP. The per-face split is now incidental cost: many well-known filenames, one lone TCP port that forces a non-derivable address and a stale-prone `.url` sidecar.

A separate component — the standalone MCP **gateway** daemon (ADR-0013/0014/0015) — keeps its standard streamable-HTTP front for arbitrary external MCP clients. That is a different thing from the gate's own MCP face and is untouched here.

## Goals / Non-Goals

**Goals:**

- Collapse the gate's faces behind one `$ATHING_DIR/gate.sock`, demultiplexed by a per-connection route preamble, so the single trust boundary of ADR-0016 is realized as a single socket.
- Make the gate's MCP face socket-only, removing the unused HTTP transport so the gate binds no TCP port at all.
- Define one route-preamble contract, shared by every gate-native client peer, that mirrors the gate's internal inbound shape.
- Preserve each face's existing post-admission lifecycle and authentication outcomes exactly.

**Non-Goals:**

- The `gate-mcp-bridge` stdio↔socket binary (the MCP analog of the hook producer). Deferred until an agent actually consumes the gate's own MCP tools; none does today.
- Any remote/networked gate transport.
- Changing how the agent CLI fires hooks (still one process exec per event).
- The standalone MCP gateway daemon's HTTP front (ADR-0014) — out of scope, unchanged.

## Decisions

### One socket, route preamble

Bind a single `UnixListener` at the derived `$ATHING_DIR/gate.sock`. Every accepted connection is read for exactly one length-prefixed **preamble** frame `{ route, session, token?, wireVersion }` using the shared codec (ADR-0009), then demultiplexed to the existing per-face handler. The preamble mirrors the gate's internal `Inbound` (route ≈ kind, session, token), so the wire envelope and the router input converge on one shape.

- _Alternative — keep N sockets:_ rejected. The faces already share a codec; distinct filenames buy nothing and contradict the single-boundary intent. No runtime cost is saved by separate listeners.
- _Alternative — N sockets inside one directory with a manifest:_ rejected. Still multiple binds and a discovery artifact; the preamble is simpler and self-describing.

### The MCP route upgrades after the preamble

For `route: Mcp`, the demux verifies the preamble token, then stops reading gate frames and hands the remaining byte stream to the MCP protocol library — exactly what the current MCP socket handler already does after its handshake. The other routes keep framing. This is a per-connection protocol upgrade (the HTTP `Upgrade`/`CONNECT` shape): one route continues in the gate codec, one switches to a sub-protocol.

- _Alternative — tunnel MCP inside gate frames:_ rejected. It would forbid using a maintained MCP library over the raw stream and re-introduce a hand-rolled wire, which gate-mcp-routing explicitly avoids.

### Socket-only MCP; the gate becomes port-free

Remove the MCP HTTP transport entirely (the transport selector, the loopback HTTP app and its admission layer, the TCP bind, and the published `.url`). With the hook face already on a socket, the gate then binds no TCP listener at all — every surface is a Unix socket at a path derived from `$ATHING_DIR`. Verified against the MCP Rust SDK: serving MCP over an arbitrary `AsyncRead + AsyncWrite` stream (a Unix socket) is first-class, and the gate already does it — so socket-only loses no MCP capability for same-host clients.

- _Alternative — keep the HTTP transport:_ rejected now. It has zero consumers, yet forces a TCP port, an HTTP server stack, a loopback-origin check, and a stale-prone sidecar. Dropping it while unused is the cheapest possible moment.
- _Distinction:_ this is the gate's own MCP face, not the standalone MCP gateway daemon (ADR-0014), whose standard HTTP front for external clients is unchanged.

### Admin: credential-in-demux, not a physical wall

`Admin` becomes a route on the shared socket but is governed by a centralized **route → credential** policy: `Hook`/`Tool`/`Mcp` require a valid session token; `Admin` requires the admin token (distinct from any session token); `Subscribe` requires none. A connection holding only a session token is refused on the `Admin` route. The privileged registry-mutate path is thus separated by credential, not by socket file.

- _Alternative — keep `gate-admin.sock` physically separate:_ a defensible conservative option; it preserves defense-in-depth as a second wall. Rejected in favor of the single-socket goal, on the condition that the route→credential policy is centralized in one place and guarded by a negative test (a session token must not satisfy `Admin`). This trade-off is recorded in the ADR.

### Adapter family: shared library, separate processes

Gate-native peers share a small gate-ingress client library (connect, write preamble, frame codec) plus the preamble contract in `contracts-rs`. They remain **separate processes** because their lifecycles differ and cannot be unified:

```
  hook producer    route Hook       spawned per event, write, exit   (one-shot)
  mcp bridge*      route Mcp        spawned once, pump full-duplex   (persistent)  *future
  subscribe peer   route Subscribe  long-lived read stream           (consumer)
```

The agent forces these shapes (a hook is one process exec per event; an MCP server is one persistent stdio process), so folding them into one binary is impossible. Share the seam (preamble + codec), not the process.

## Risks / Trade-offs

- **Route/credential confusion in the demux could span trust tiers** (e.g. a session token reaching `Admin`). → Centralize the route→credential mapping in one function; cover it with negative tests asserting each route refuses every credential except its own, especially `Admin`.
- **MCP framing→upgrade handoff on a shared socket** — the demux must consume exactly one preamble frame and hand the *same* stream (no buffered residue) to the MCP library. → The codec already decodes incrementally across partial reads; read one frame, then pass the live stream object onward, as the current MCP socket handler already does.
- **`Subscribe` carries no token on a socket now shared with token routes.** → The route→credential policy makes "no token" an explicit, audited decision for `Subscribe` only; binding `Subscribe` to a session token is a possible hardening follow-up (Open Questions).
- **Breaking, pre-v1, lockstep rewire** — producer, consumers, and gate must move together. → Land atomically in one change; no compatibility shim, consistent with prior pre-v1 gate migrations.
- **Dropping `axum`/TCP from the gate crate may be incomplete** if another path uses it. → Confirm during tasks that the MCP HTTP face is the last `axum`/TCP consumer before removing the dependency.

## Migration Plan

Pre-v1: no backward compatibility is kept. Land as one change:

1. Add the route-preamble envelope + `Route` enum to `contracts-rs`.
2. Replace the five per-face binds with one `gate.sock` listener + preamble demux; route each existing handler from the demux; implement the route→credential policy; upgrade the `Mcp` route after preamble.
3. Remove the MCP HTTP transport, its config selectors, and `gate-mcp.url`.
4. Re-wire each client peer to open with its route preamble (hook producer, subscribe consumers, tool client, admin client) and update their tests.
5. Rewrite `docs/services.md` to the single-socket-by-route model; record the ADR.

Rollback: revert the single change.

## Open Questions

- Should `Subscribe` require a per-session token as a hardening step, rather than remaining open? It would tighten the one no-credential route now sharing the socket.
- When the agent eventually needs the gate's own MCP tools, the deferred `gate-mcp-bridge` (stdio↔socket) gives it a standard stdio MCP front backed by the socket — the reconciliation with ADR-0014's "standard MCP front for agent-facing access" principle. Confirm that is the intended future path before the bridge is built.
- No in-force ADR is contradicted; this change deepens ADR-0016 and reuses ADR-0009. The new ADR is additive (no supersession).
