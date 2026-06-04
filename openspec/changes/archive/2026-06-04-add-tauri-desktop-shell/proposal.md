## Why

The product ships today as a browser SPA backed by a network server (`apps/server`) that hosts
the agent engine and relays bytes over a WebSocket to a local PTY daemon. For a single-user,
bring-your-own-login desktop tool that topology is wrong: it forces a browser, a long-lived
network server, and a JSON-over-WebSocket hop whose data path even re-encodes raw terminal
bytes as number arrays. Once the engine no longer performs its own I/O (it depends on injected
`DaemonTransport` and `FileSource` ports), the agent loop can run directly inside a native
desktop web view, with the native shell providing those ports. That removes the browser, the
network server, and the relay — leaving a native window, a generic PTY daemon, and the engine
running in the renderer over a native bridge.

This change depends on the engine I/O ports being in place; it supplies the desktop
implementations of those ports and the native shell around them.

## What Changes

- Add a native desktop application built with the Tauri v2 framework (Rust core + system
  web view) as a new package `apps/desktop`.
- Run the agent engine and adapter **inside the web view** (the existing renderer becomes a
  full host of the agent loop), driving them through the injected I/O ports rather than a
  backend agent process. No headless agent server runs on the desktop path.
- Implement the engine's port contracts natively in the Rust core: a daemon-transport
  implementation that bridges the renderer to the generic PTY daemon over its existing
  binary protocol, and a file-read implementation for transcript reads. Raw bytes are carried
  end-to-end with no re-encode.
- Bundle the existing Bun PTY **daemon** as a Tauri sidecar (the only background process on the
  desktop path); the Rust core spawns, supervises, and tears it down, and performs the startup
  bootstrap the host now owns (agent-binary resolution, version check, hook-command preparation).
- Move user preferences and the session registry to a native local store owned by the Rust
  core, replacing `apps/server`'s role for those concerns on the desktop path.
- Reuse `apps/ui` (React Router + `@xterm/xterm`) as the renderer; add the `@xterm/addon-webgl`
  GPU rendering backend, and select the native port-backed transport on desktop versus the
  network transport on web behind a single abstraction.
- Switch the renderer to a static client-only (SPA) build so the native web view can serve it
  from the bundle with no server-render runtime; the same static build serves the web deployment.
  Remove the default scaffold renderer from `apps/desktop` and repoint the native config
  (production-frontend path, dev URL, before-dev/before-build commands) at the shared renderer, so
  the desktop carries the native shell + build wiring only — no rival renderer.
- **BREAKING** (desktop build only): there is no network server and no agent backend process on
  the desktop path. `apps/server` remains the agent/API/WebSocket backend for the web deployment.
- **BREAKING** (web deployment): the renderer is no longer server-rendered; it is served as static
  SPA assets and talks to `apps/server` over `/api` and `/ws` (the topology it already assumes).
- Add the Rust toolchain and Tauri tooling to the build/release pipeline; produce signed bundles
  for macOS and Linux (v1 platform scope).

## Capabilities

### New Capabilities

- `desktop-shell`: native desktop application lifecycle — window management, startup/shutdown,
  loading the web-view renderer, and signed bundling for macOS and Linux.
- `desktop-daemon-host`: the native core spawning, supervising, and gracefully terminating the
  generic PTY daemon sidecar, plus the one-time startup bootstrap the host now owns
  (agent-binary resolution, version verification, hook-command preparation), honoring the
  existing daemon adopt/handoff manifest.
- `desktop-native-ports`: native implementations of the engine's three sdk port contracts
  (daemon-transport, file-read, logger), exposed to the in-web-view engine — bridging the
  daemon's binary protocol with raw, ordered, back-pressured byte delivery, serving transcript
  reads, and capturing diagnostics. Together they form a `platform-tauri` layer, the web-view
  sibling of `@athing/platform-bun` implementing the identical contracts.
- `desktop-engine-runtime`: hosting the agent engine and adapter inside the web view over the
  injected ports (no backend agent process), including GPU-accelerated terminal rendering and
  selecting the native versus network transport behind one abstraction.
- `desktop-app-data`: a native local store for user preferences and the session registry,
  owned by the desktop core.
- `desktop-renderer-build`: the renderer's static SPA build wiring — one client-only build serving
  both hosts, the desktop frontend resolving to that build (dev server in development, static
  output in production), removal of the scaffold-template renderer, and client-side routing under
  the web-view asset origin.

### Modified Capabilities

<!-- The generic PTY daemon, the engine's agent-loop behavior, and the engine I/O port contracts
     keep their requirements. The renderer's render mode changes (SSR -> static SPA) and is
     captured by the new desktop-renderer-build capability; no separate renderer behavior spec is
     modified. -->

## Impact

- **Depends on**: `refactor-engine-io-ports` (engine obtains its daemon link, transcript reads,
  and logger through injected contracts; `cwd` required per session) **and**
  `refactor-sdk-web-api` (the sdk wire codec + snapshot renderer use Web byte APIs, so they load
  in a web view). Both have landed.
- **New package**: `apps/desktop` (Tauri v2 — Rust core, native port implementations, daemon
  supervision, native app-data store, the renderer bridge).
- **New dependency/toolchain**: Rust + Cargo, Tauri CLI and crates; CI/release gains native
  bundling and macOS/Linux code-signing.
- **Renderer**: `apps/ui` hosts the engine + adapter, gains the native transport implementation
  and the `@xterm/addon-webgl` dependency; switches to a static client-only (SPA) build
  (`ssr: false`); no component rewrite.
- **`apps/desktop` scaffold**: the default Tauri scaffold renderer (template entry document,
  sample root component, sample assets, scaffold-local bundler/dev-server config, and renderer
  deps) is removed; the package retains `src-tauri` + native build config pointed at `apps/ui`.
- **Reused unchanged**: `packages/daemon` (incl. `node-pty`, snapshot/handoff/flow-control) and
  the engine's agent-loop logic.
- **`apps/server`**: stays the agent/API/WebSocket backend; no longer server-renders the
  renderer. The web deployment serves `apps/ui` as static SPA assets (the renderer's serve entry
  and container image switch to static serving). Desktop build does not use `apps/server`.
- **Platform scope**: macOS and Linux for v1; the system web view is WKWebView on macOS and
  WebKitGTK on Linux.
- **Reliability contract (ADR-0007)**: daemon supervision and shutdown must honor graceful
  shutdown and timeouts; the native ports must preserve backpressure and raw-byte fidelity.
