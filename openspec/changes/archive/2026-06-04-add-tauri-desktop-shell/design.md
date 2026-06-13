## Context

After the engine I/O ports change (`refactor-engine-io-ports`), the engine no longer performs
its own I/O: it drives the daemon through an injected `DaemonTransport` and reads transcripts
through an injected `FileSource`, and the host supplies startup-resolved values (connected
transport, file source, resolved agent command, hook command). The wire framing codec is a pure
function in `@athing/sdk`, so it runs in any JavaScript host — including a web view.

That unlocks a native desktop topology with no backend agent process. The renderer (`apps/ui`)
hosts the engine and adapter directly; a Tauri v2 shell (Rust core + system web view) supplies
the desktop implementations of the ports and the startup bootstrap. The only background process
is the existing generic PTY daemon, bundled as a sidecar. `apps/server` remains the web host and
is untouched.

This supersedes the earlier headless-core-sidecar approach: there is no second Bun process and
no local agent server on the desktop path.

## Goals / Non-Goals

**Goals:**

- Ship a native macOS/Linux desktop app hosting the existing renderer, with no browser and no
  network server.
- Run the agent engine + adapter inside the web view over the injected ports.
- Carry raw PTY bytes end-to-end (no JSON number-array re-encode), ordered and back-pressured.
- Keep the Rust core thin; reuse the generic daemon and the engine's agent loop unchanged.
- Honor the reliability contract (ADR-0007): graceful daemon shutdown, timeouts, typed errors,
  backpressure.

**Non-Goals:**

- Windows support (v1 is macOS/Linux).
- Rewriting the daemon, the engine, or the terminal renderer in Rust.
- Porting the wire framing codec to Rust (it stays pure JS in the web view).
- Changing `apps/server` or the web deployment.

## Decisions

### D1. Engine + adapter run in the web view over the injected ports

The renderer hosts the engine and adapter and constructs `createEngine(deps)`, where
`deps = { transport, fileSource, logger, hooksSocketPath }` — the three sdk port contracts
(`DaemonTransport`, `FileSource`, `Logger`) plus the hook socket path, all implemented for the
web-view host. The renderer also supplies `cwd` on every session start (the engine now requires
it; a missing `cwd` is a typed error). There is no backend agent process.

_Why:_ after `refactor-engine-io-ports` the engine is platform-free, and after
`refactor-sdk-web-api` the sdk wire codec and snapshot renderer use Web byte APIs — so the whole
agent loop loads in a web view. Hosting it there removes the backend server and relay.
_Alternative:_ a headless Bun agent sidecar (the earlier approach) — rejected, it reintroduces a
second process and a local relay for no benefit on a single-user desktop.

### D2. The Rust core is a dumb byte bridge to the daemon socket; framing stays in the web view

The web view's `DaemonTransport` implementation reuses the sdk codec to encode/decode frames and
moves _raw bytes_ across the Tauri boundary: a Tauri Channel for the daemon->renderer byte stream
and an `invoke` command for renderer->daemon writes. The Rust core simply forwards those bytes
to/from the daemon's Unix socket — it never parses a frame.

_Why:_ keeps the codec single-sourced in sdk (the web-view impl differs from the Bun impl only in
its byte carrier), and keeps the Rust core trivial — no protocol port. _Alternatives:_ (a) Rust
implements the frame codec and exposes structured messages — rejected, duplicates the codec and
adds Rust protocol logic; (b) expose the daemon socket to the web view directly — impossible, web
views cannot open Unix sockets.

### D3. `FileSource` served by Rust file-read commands

The web view's `FileSource` implementation calls Rust commands (`size`, `read(path, offset,
length)`) over `invoke`; the Rust core performs the filesystem reads. Transcript reads happen on
hooks, not on the hot byte path, so per-call IPC is acceptable.

_Why:_ the web view cannot read the filesystem; the native core can. _Alternative:_ stream the
transcript over a Channel — unnecessary, reads are delta-sized and infrequent.

### D3a. `Logger` served by the native core; impls mirror `@athing/platform-bun`

The renderer supplies a `Logger` impl (console + an optional forward to the Rust core for
file/diagnostic capture). The three port impls together form a `platform-tauri` layer — the
web-view sibling of `@athing/platform-bun`: both implement the same sdk port contracts
(`DaemonTransport`, `FileSource`, `Logger`), differing only in host (Bun socket/fs vs Tauri
Channel/`invoke`).

_Why:_ the engine needs all three injected ports (the refactor added `Logger`); framing the
Tauri impls as the parallel `platform-*` keeps one contract, two hosts. _Alternative:_ a webview
logger that writes nowhere — rejected, loses desktop diagnostics.

### D4. Rust core supervises the daemon sidecar and performs startup bootstrap

The Rust core spawns/adopts and supervises the generic PTY daemon (honoring the existing
adopt/handoff manifest) and performs the one-time bootstrap the host now owns: resolve the agent
binary, verify its version, and prepare the hook command. It exposes the resolved values to the
web view at startup; the renderer constructs the ports and calls `createEngine(deps)`. On window
close, the Rust core terminates the daemon it owns.

_Why:_ spawning/owning an OS process and probing the environment are native-host concerns (the
ports refactor moved them out of the engine). _Alternative:_ the web view shells out for these —
impossible, no process/spawn access in a web view.

### D5. Sidecar packaging: ship the Bun runtime + daemon script, not a compiled binary

A compiled Bun binary breaks PTY spawn (`posix_spawnp`), documented in the daemon's
`build-wrapper.ts`. Ship the `bun` executable as the Tauri `externalBin` (per target triple) plus
the bundled daemon script as a resource; the sidecar invocation passes the entry script to `bun`.
Only the daemon is a sidecar now (one process).

_Why:_ preserves the working `node-pty` spawn path. _Alternatives:_ `bun build --compile` —
rejected (breaks PTY spawn); assume system Bun — rejected (environment dependence); thin native
launcher per triple — viable fallback.

### D6. User preferences and session registry move to a native app-data store

The Rust core owns a native local store for user preferences and the session registry
(sessionId -> cwd, used for reconnect). The renderer reads/writes it over `invoke`. This replaces
`apps/server`'s `bun:sqlite` registry on the desktop path.

_Why:_ desktop persistence is a native-shell concern and removes the last reason for a Bun
backend on desktop. _Alternative:_ keep a Bun process just for sqlite — rejected, defeats the
no-backend goal; the web build keeps its own server-side registry.

### D7. Renderer: pluggable transport + GPU rendering

`apps/ui` selects the native transport (Channel + `invoke`) on desktop and the WebSocket
transport on web behind one `Transport` abstraction; components are unchanged. Add
`@xterm/addon-webgl` with a canvas fallback.

_Why:_ one renderer, two carriers; GPU glyph rendering is the largest renderer-side throughput
lever. _Alternative:_ per-message Tauri events instead of a Channel — higher overhead, no
ordering guarantee for a byte stream.

### D8. Renderer ships as a static SPA; one build serves web and desktop

The renderer switches to a client-only single-page build (SSR off). It emits static client
assets that load over a custom asset origin with no server-render runtime, so the native web
view serves them directly from the bundle. The same static build serves the web deployment;
`apps/server` becomes the API + WebSocket backend only (no longer a render host), which the
existing renderer already assumes (it talks to the backend over `/api` and `/ws`).

_Why:_ a native web view loads files over a custom protocol — it cannot run a Node server-render
process, so an SSR renderer cannot be the desktop frontend. The renderer is already a client of
`apps/server` over WebSocket; a client-only build is the natural single source and avoids a
renderer fork. _Alternatives:_ (a) keep SSR for web and emit a separate SPA build for desktop via
an env-driven config — rejected, two build modes for no product benefit once the renderer is a
pure client; (b) point the desktop frontend at a live render server even in production — rejected,
defeats an offline-capable native app and reintroduces a backend on the desktop path.

_Impact:_ the web deployment stops being server-rendered. The renderer's server-serve entry and
its container image change to static asset serving; no component rewrite.

### D9. Desktop package is the native shell + build wiring only; the scaffold template is removed

`apps/desktop` keeps the native core (`src-tauri`) and the build configuration that points at the
shared renderer. The default scaffold renderer — its template entry document, sample root
component, sample assets, and the scaffold-local bundler/dev-server config and renderer
dependencies — is deleted. The desktop frontend resolves to the shared renderer's static client
output; in development it loads the shared renderer's dev server.

Concretely the native config is repointed: the production-frontend path targets the shared
renderer's static client build output; the dev URL targets the shared renderer's dev server; the
before-dev and before-build commands drive the shared renderer's dev and build, not a
scaffold-local app.

_Why:_ the change's "one renderer, no fork" decision (D1/D7) means the desktop package must not
carry a second renderer. The scaffold template is starter noise that would diverge. _Alternative:_
keep a thin desktop renderer that imports the shared renderer as a package — rejected, adds a
second entry tree and a fork surface for zero benefit.

### D10. Client-side routing under the web-view asset origin

The renderer's routing runs fully client-side; deep links and reloads resolve within the web view
against the static asset origin with no server rewrite. The static build's index document is the
single entry; unknown paths resolve through the client router.

_Why:_ there is no server on the desktop path to rewrite routes to the SPA entry, so routing must
not depend on server fallbacks. _Alternative:_ a custom-protocol handler in the Rust core that
rewrites routes — unnecessary once the build is a client-only SPA with a single entry document.

## Risks / Trade-offs

- [Backpressure must survive the Channel hop: the daemon's flow-control credit/ack loop now spans
  daemon -> Rust -> Channel -> web-view engine] -> Preserve the existing credit/ack semantics across
  the transport; the web-view `DaemonTransport` returns credit as the renderer drains, exactly as
  the Bun impl does. Verify no drops/reorders under load.
- [Tauri `externalBin` expects a compiled binary per triple, but the sidecar is `bun` + script]
  -> Ship `bun` as the per-triple `externalBin` with the script as a resource; thin native
  launcher as fallback (D5).
- [Bundling the `bun` runtime inflates size and the embedded executable needs macOS
  signing/notarization] -> Accept the size cost (one runtime); add the embedded binary to signing.
- [WKWebView/WebKitGTK WebGL quirks] -> canvas fallback, feature-detect at startup (D7).
- [Per-hook transcript reads cross web-view -> Rust IPC] -> acceptable; reads are delta-sized and
  hook-frequency, off the hot byte path (D3).
- [The hook command assumes a runtime to run the notify script in the agent's environment]
  -> see Open Questions; the bundle must provide it.

## Migration Plan

1. Prerequisite: `refactor-engine-io-ports` landed (engine consumes injected ports).
2. Add the web-view `DaemonTransport` implementation in `apps/ui`: reuse the sdk codec, carry raw
   bytes over a Channel (inbound) and `invoke` (outbound).
3. Add the web-view `FileSource` implementation backed by Rust file-read commands.
4. Add the app-data accessor (preferences + session registry) over `invoke`.
5. Add the `Transport` abstraction and `@xterm/addon-webgl` to `apps/ui`.
6. Scaffold `apps/desktop` (Tauri v2): byte bridge to the daemon socket, file-read commands,
   app-data store, daemon supervision + bootstrap (resolve/version/hook-command), sidecar
   packaging (D5), window-close shutdown.
7. Renderer bootstrap: `invoke` to fetch resolved values, construct the ports, call
   `createEngine(deps)`, run the engine in the web view.
8. Release pipeline: Rust toolchain, Tauri bundling, macOS/Linux signing/notarization.

Rollback: the web build (`apps/ui` + `apps/server`) stays fully functional; the desktop app is
additive. Reverting means dropping `apps/desktop` and the desktop port implementations.

## Open Questions

- The hook command assumes a runtime is available to run the notify script when the _agent_
  fires a hook. In a packaged desktop app that runtime may not be on the user's PATH. The bundle
  must ship the notify script and a runtime (or rewrite the hook command). Resolve before release.
- Exact Tauri `externalBin` packaging for a Bun-script sidecar: ship `bun` directly, or a thin
  native launcher per triple (D5)?
- Confirm the flow-control credit/ack loop is preserved byte-for-byte across the Channel hop
  under sustained high-throughput output (D2 / backpressure risk).
- Does `reconnect` need anything beyond `cwd` from the native app-data store, or is the daemon's
  live-session list sufficient?
- Web Crypto secure-context: the engine generates ids/tokens via `crypto.randomUUID()` /
  `getRandomValues()`. `randomUUID` requires a secure context — confirm Tauri v2's custom-protocol
  web view qualifies (it should); else provide a secure-context-independent id source.
