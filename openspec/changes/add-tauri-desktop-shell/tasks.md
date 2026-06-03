## 1. Renderer static SPA build (desktop-renderer-build)

- [x] 1.1 Set `ssr: false` in `apps/ui/react-router.config.ts`; confirm `react-router build` emits a static client bundle (`build/client/`) with a single entry document
- [x] 1.2 Verify all routes are client-only (no `loader`/`action` requiring a server); convert any server-data fetch to client fetch against `apps/server` `/api` + `/ws`
- [x] 1.3 Confirm client-side routing (deep links + reload) resolves from the static entry document with no server rewrite
- [x] 1.4 Switch the web deployment to serve `apps/ui` static assets: replace the `react-router-serve` `start` script and the `apps/ui/Dockerfile` with static serving of `build/client/`
- [x] 1.5 Confirm the web deployment still reaches `apps/server` over `/api` + `/ws` after the SPA switch

## 2. Renderer transport abstraction + native port clients (desktop-engine-runtime, desktop-native-ports)

- [ ] 2.1 Add a `Transport` abstraction in `apps/ui` selecting native vs network carrier; keep the WebSocket transport as the network implementation
- [ ] 2.2 Detect host (desktop vs web) at startup and select the transport accordingly, with identical user-facing behavior
- [ ] 2.3 Implement the web-view `DaemonTransport` (sdk port): reuse the sdk wire codec; carry inbound daemon bytes over a Tauri Channel and outbound writes over `invoke`, raw bytes only
- [ ] 2.4 Preserve the daemon flow-control credit/ack loop in the web-view transport: return consumption credit as the renderer drains, no drops/reorders (ADR-0007 backpressure)
- [ ] 2.5 Implement the web-view `FileSource` (sdk port) over Rust `size`/`read(path, offset, length)` commands; surface absent file distinctly
- [ ] 2.6 Implement the web-view `Logger` (sdk port): console plus optional forward to the native core
- [ ] 2.7 Add `@xterm/addon-webgl` with feature-detect and canvas fallback, no loss of output fidelity

## 3. Desktop package cleanup + native config wiring (desktop-renderer-build, desktop-shell)

- [x] 3.1 Delete the scaffold renderer from `apps/desktop`: `src/`, `index.html`, `vite.config.ts`, `public/{vite,tauri}.svg`, and scaffold renderer deps in `package.json`
- [x] 3.2 Repoint `src-tauri/tauri.conf.json`: `frontendDist` -> `apps/ui` static client output; `devUrl` -> `apps/ui` dev server; `beforeDevCommand`/`beforeBuildCommand` -> `apps/ui` dev/build
- [x] 3.3 Reduce `apps/desktop/package.json` to native shell + build orchestration scripts (`tauri`, dev/build that drive `apps/ui`); remove `react`/`react-dom`/vite renderer deps
- [x] 3.4 Set product window title/identifier and a sensible default window size in `tauri.conf.json`; replace placeholder favicon reference
- [x] 3.5 Verify `bun run tauri dev` opens a native window loading the `apps/ui` dev server with live reload

## 4. Native core: platform-tauri ports (desktop-native-ports)

- [ ] 4.1 Implement the Rust byte bridge to the daemon's local socket: forward renderer outbound bytes verbatim, deliver daemon output bytes back over a Channel; never parse a frame
- [ ] 4.2 Wire the bridge to preserve the daemon's flow-control loop end-to-end (daemon -> Rust -> Channel -> renderer), no drops/reorders under sustained output
- [ ] 4.3 Implement Rust file-read commands (`size`, `read(path, offset, length)`) returning bytes and reporting an absent file distinctly
- [ ] 4.4 Implement the native diagnostic channel backing the renderer `Logger` forward
- [ ] 4.5 Declare Tauri capabilities/permissions for the Channel, file-read, and app-data `invoke` commands in `src-tauri/capabilities/`

## 5. Native core: daemon supervision + startup bootstrap (desktop-daemon-host)

- [ ] 5.1 On startup, spawn the generic PTY daemon when no live, compatible daemon is recorded; record ownership in the adopt/handoff manifest
- [ ] 5.2 Adopt an already-recorded live, compatible daemon instead of spawning a duplicate
- [ ] 5.3 Establish daemon reachability before enabling session interaction
- [ ] 5.4 Perform startup bootstrap: resolve the agent executable, verify its version, prepare the hook command; expose resolved values to the renderer
- [ ] 5.5 Surface a typed version-unsupported error before accepting sessions when the agent version is out of range
- [ ] 5.6 Detect unexpected daemon exit and surface a typed lost-connection error to the renderer
- [ ] 5.7 On exit, gracefully terminate an owned daemon (wait for shutdown to begin); leave an adopted daemon running (ADR-0007)

## 6. Native core: app-data store (desktop-app-data)

- [ ] 6.1 Implement a native local store for user preferences (read/write over `invoke`), persisting across restarts
- [ ] 6.2 Implement the session registry (sessionId -> cwd) over `invoke`, supplying cwd on reconnect
- [ ] 6.3 Reconcile the registry against the daemon's live sessions on startup, removing stale entries

## 7. Sidecar packaging (desktop-daemon-host, design D5)

- [ ] 7.1 Ship the `bun` executable as the Tauri `externalBin` per target triple (macOS, Linux); bundle the daemon entry script as a resource
- [ ] 7.2 Invoke the sidecar by passing the daemon entry script to `bun` (no `--compile`, preserving the `node-pty` spawn path)
- [ ] 7.3 Resolve the hook-command runtime in the packaged bundle (ship notify script + runtime, or rewrite the hook command) so agent-fired hooks run off-PATH

## 8. Renderer bootstrap: run the engine in the web view (desktop-engine-runtime)

- [ ] 8.1 On desktop startup, `invoke` to fetch the native-resolved values (transport handle, file-read, logger, hook command)
- [ ] 8.2 Construct the ports and call `createEngine(deps)` in the renderer before starting any session
- [ ] 8.3 Supply `cwd` on every session start/reconnect; a missing `cwd` surfaces a typed error rather than starting
- [ ] 8.4 Confirm Web Crypto (`crypto.randomUUID`/`getRandomValues`) works in the Tauri web view secure context; provide a fallback id source if not

## 9. Release pipeline (desktop-shell)

- [ ] 9.1 Add the Rust toolchain + Tauri CLI to the build/release pipeline (turbo + CI)
- [ ] 9.2 Produce signed, installable bundles for macOS and Linux; add the embedded `bun` binary to macOS signing/notarization
- [ ] 9.3 Verify a release bundle launches offline, opens a native window, drives a session, and shuts the daemon down gracefully on close

## 10. Verification (cross-cutting)

- [ ] 10.1 Backpressure load test: sustained high-throughput daemon output stays ordered, byte-for-byte, with no drops across the Channel hop (ADR-0007)
- [ ] 10.2 Raw-byte fidelity test: no ANSI stripping / UTF-8 re-decode end-to-end on the desktop path
- [ ] 10.3 Confirm web deployment (static `apps/ui` + `apps/server`) remains fully functional after the SPA switch
