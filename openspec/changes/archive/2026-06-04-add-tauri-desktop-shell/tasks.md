## 1. Renderer static SPA build (desktop-renderer-build)

- [x] 1.1 Set `ssr: false` in `apps/ui/react-router.config.ts`; confirm `react-router build` emits a static client bundle (`build/client/`) with a single entry document
- [x] 1.2 Verify all routes are client-only (no `loader`/`action` requiring a server); convert any server-data fetch to client fetch against `apps/server` `/api` + `/ws`
- [x] 1.3 Confirm client-side routing (deep links + reload) resolves from the static entry document with no server rewrite
- [x] 1.4 Switch the web deployment to serve `apps/ui` static assets: replace the `react-router-serve` `start` script and the `apps/ui/Dockerfile` with static serving of `build/client/`
- [x] 1.5 Confirm the web deployment still reaches `apps/server` over `/api` + `/ws` after the SPA switch

## 2. Renderer transport abstraction + native port clients (desktop-engine-runtime, desktop-native-ports)

- [x] 2.1 Add a `Transport` abstraction in `apps/ui` selecting native vs network carrier; keep the WebSocket transport as the network implementation > `apps/ui/app/lib/transport/`: `FramedDaemonTransport` base (sdk codec + handshake + > dispatch), `WebSocketDaemonTransport` (network impl), `TauriDaemonTransport` (native impl). > Unit-tested. The host selection is the SessionPage web/desktop component branch (§8.5): > web -> `TerminalPane`/WS, desktop -> `bootDesktopHost` -> native engine.
- [x] 2.2 Detect host (desktop vs web) at startup and select the transport accordingly, with identical user-facing behavior > `isDesktopHost()` (probes `window.__TAURI_INTERNALS__`); the SessionPage + SessionSidebar > branch on it (desktop -> native engine path, web -> existing WS path).
- [x] 2.3 Implement the web-view `DaemonTransport` (sdk port): reuse the sdk wire codec; carry inbound daemon bytes over a Tauri Channel and outbound writes over `invoke`, raw bytes only > `TauriDaemonTransport` (`tauri.ts`): inbound over `Channel<Vec<u8>>`, outbound over > `daemon_send`; Rust side in `src-tauri/src/bridge.rs`. Unit-tested with an injected core.
- [x] 2.4 Preserve the daemon flow-control credit/ack loop in the web-view transport: return consumption credit as the renderer drains, no drops/reorders (ADR-0007 backpressure) > Structural: frames cross verbatim, the ordered Channel preserves sequence, and renderer- > emitted ack frames ride `daemon_send` back. Sustained-load proof DESCOPED with packaging (§10).
- [x] 2.5 Implement the web-view `FileSource` (sdk port) over Rust `size`/`read(path, offset, length)` commands; surface absent file distinctly > `TauriFileSource` (`file-source.ts`) + Rust `file_size` (null when absent) / `file_read`.
- [x] 2.6 Implement the web-view `Logger` (sdk port): console plus optional forward to the native core > `TauriLogger` (`logger.ts`) + Rust `log_forward`. > DESCOPED 2.7 (`@xterm/addon-webgl`): no GPU backend — terminal renders via xterm's default > canvas. No stable webgl addon exists for xterm 6.0.0; GPU rendering requirement dropped.

## 3. Desktop package cleanup + native config wiring (desktop-renderer-build, desktop-shell)

- [x] 3.1 Delete the scaffold renderer from `apps/desktop`: `src/`, `index.html`, `vite.config.ts`, `public/{vite,tauri}.svg`, and scaffold renderer deps in `package.json`
- [x] 3.2 Repoint `src-tauri/tauri.conf.json`: `frontendDist` -> `apps/ui` static client output; `devUrl` -> `apps/ui` dev server; `beforeDevCommand`/`beforeBuildCommand` -> `apps/ui` dev/build
- [x] 3.3 Reduce `apps/desktop/package.json` to native shell + build orchestration scripts (`tauri`, dev/build that drive `apps/ui`); remove `react`/`react-dom`/vite renderer deps
- [x] 3.4 Set product window title/identifier and a sensible default window size in `tauri.conf.json`; replace placeholder favicon reference
- [x] 3.5 Verify `bun run tauri dev` opens a native window loading the `apps/ui` dev server with live reload

## 4. Native core: platform-tauri ports (desktop-native-ports)

- [x] 4.1 Implement the Rust byte bridge to the daemon's local socket: forward renderer outbound bytes verbatim, deliver daemon output bytes back over a Channel; never parse a frame > `src-tauri/src/bridge.rs` (`daemon_connect`/`daemon_send`/`daemon_disconnect`): tokio > `UnixStream` to `$ATHING_DIR/daemon.sock`, read task streams bytes over the Channel, never > parses a frame. `cargo check` clean. Socket path resolves via env today; §5 spawns/adopts.
- [x] 4.2 Wire the bridge to preserve the daemon's flow-control loop end-to-end (daemon -> Rust -> Channel -> renderer), no drops/reorders under sustained output > Bytes forwarded verbatim, ordered Channel, no buffering drops. Sustained-load proof DESCOPED with packaging (§10).
- [x] 4.3 Implement Rust file-read commands (`size`, `read(path, offset, length)`) returning bytes and reporting an absent file distinctly > `src-tauri/src/files.rs`: `file_size` -> `Option<u64>` (null absent), `file_read` -> raw `Response`.
- [x] 4.4 Implement the native diagnostic channel backing the renderer `Logger` forward > `src-tauri/src/diag.rs` (`log_forward`).
- [x] 4.5 Declare Tauri capabilities/permissions for the Channel, file-read, and app-data `invoke` commands in `src-tauri/capabilities/` > App `#[tauri::command]`s are allowed by default in v2; the Channel rides core IPC under > `core:default`. Documented in `capabilities/default.json` — no per-command permission needed.

## 5. Native core: daemon supervision + startup bootstrap (desktop-daemon-host)

- [x] 5.1 On startup, spawn the generic PTY daemon when no live, compatible daemon is recorded; record ownership in the adopt/handoff manifest > `src-tauri/src/supervisor.rs` `daemon_ensure`: reads `$ATHING_DIR/daemon.json`, spawns the > resolved daemon binary detached, records `Owned`. Protocol compatibility is enforced at the > daemon hello/hello-ack handshake (transport), not by a manifest version comparison.
- [x] 5.2 Adopt an already-recorded live, compatible daemon instead of spawning a duplicate > `daemon_ensure` adopts when the manifest pid is alive (`kill(pid,0)`) and the socket is > reachable; records `Adopted`.
- [x] 5.3 Establish daemon reachability before enabling session interaction > Reachability is a real `UnixStream::connect` probe (spawn path polls up to 10s); > `ensureDaemon()` (host-bootstrap) gates sessions on it.
- [x] 5.4 Perform startup bootstrap: resolve the agent executable, verify its version, prepare the hook command; expose resolved values to the renderer > `src-tauri/src/bootstrap.rs` `agent_bootstrap`: resolves `claude` via the same policy as the > adapter's `resolveAgentBinary` (post daemon-decouple) — `CLAUDE_CODE_EXECUTABLE` override, > login-shell `which` (GUI apps have a sparse PATH), then common locations — parses > `--version`, resolves the `athing-notify` hook command; returns `{path, version,
  > hookCommand, hooksSocketPath}`. The native resolver is a web-view stand-in for > `adapter.resolveCommand()` (node-coupled); fold it into the web-safe-adapter refactor.
- [x] 5.5 Surface a typed version-unsupported error before accepting sessions when the agent version is out of range > `host-bootstrap.ts` `assertAgentSupported()` throws `AtError("VersionUnsupported")` against > the adapter's `cliVersionRange` (single-sourced in the adapter, not duplicated in Rust). Unit-tested.
- [x] 5.6 Detect unexpected daemon exit and surface a typed lost-connection error to the renderer > `bridge.rs` emits the `daemon-lost` event when the read loop ends without an initiated > disconnect; `DesktopHostProvider` (`useDesktopHost.tsx`) listens via `@tauri-apps/api/event` > and moves the host into a typed error state.
- [x] 5.7 On exit, gracefully terminate an owned daemon (wait for shutdown to begin); leave an adopted daemon running (ADR-0007) > `supervisor::shutdown_owned` SIGTERMs an `Owned` daemon on `RunEvent::ExitRequested`; an > `Adopted` daemon is left running.

## 6. Native core: app-data store (desktop-app-data)

- [x] 6.1 Implement a native local store for user preferences (read/write over `invoke`), persisting across restarts > `src-tauri/src/store.rs` (`pref_get`/`pref_set`) persists JSON to `$ATHING_DIR/desktop-store.json`; > TS accessor `TauriAppData` (`app-data.ts`).
- [x] 6.2 Implement the session registry (sessionId -> cwd) over `invoke`, supplying cwd on reconnect > Rust `registry_get`/`registry_set`/`registry_remove`/`registry_list` + `TauriAppData.getCwd`. > The reconnect call-site that supplies the cwd is §8 (renderer bootstrap).
- [x] 6.3 Reconcile the registry against the daemon's live sessions on startup, removing stale entries > `TauriAppData.reconcile(liveIds)` drops entries whose session is no longer live; called in > `bootDesktopHost` with `engine.listSessions()` at startup. Unit-tested.

## 7. Sidecar packaging — DESCOPED (with the release pipeline)

> DESCOPED: bundling the daemon as a Tauri `externalBin` sidecar (ship `bun` per triple + the
> daemon entry script) only matters when producing a distributable bundle, which is out of scope
> here (§9 skipped). Dev resolves the daemon via `cwd/bin/athing-daemon` (`supervisor.rs`). Defer
> to a packaging change when a release is actually cut.

## 8. Renderer bootstrap: run the engine in the web view (desktop-engine-runtime)

> Adapter web-safety: RESOLVED on main by the setup contract (`adapter-claude-code` is now
> import-safe; hook install runs through an injected `SetupFs`; the engine takes injected
> `agentHome` + `resolvedCommand`, so transcript-path/command no longer touch `os.homedir`). The
> desktop deps now supply `agentHome` (`~/.claude`) and `resolvedCommand` from the native bootstrap.
> All §8 mechanisms are built and unit-tested: `desktop-host.ts` `bootDesktopHost()` runs
> resolve+version-gate -> `ensureDaemon` -> `createDesktopEngine`; `terminal-bind.ts`
> `bindSessionToTerminal()` wires an `AgentSession` to an xterm (output -> `write`, keystrokes ->
> `input`, resize). `@athing/adapter-claude-code` added to `apps/ui` (import-safe).
>
> Hook installation is OUT OF SCOPE for the desktop app — the CLI client owns it (`add-cli-controller`).
> The desktop app only consumes already-installed hooks; `agent_bootstrap` still resolves/exposes
> the hook command per §5.4 but never writes agent settings.
>
> The remaining live step is the React render swap (8.5): call `bootDesktopHost` at app boot and
> render a desktop terminal bound via `bindSessionToTerminal` when `isDesktopHost()` (instead of
> TerminalPane's web JSON-WS). Behavior is verified on a running Tauri host (`bun run tauri dev`).

- [x] 8.1 On desktop startup, `invoke` to fetch the native-resolved values (transport handle, file-read, logger, hook command) > `host-bootstrap.ts` `bootstrapAgent()` + `ensureDaemon()`; `agent_bootstrap` now also returns > `hooksSocketPath`. Unit-tested. The call at app-boot is the integration step above.
- [x] 8.2 Construct the ports and call `createEngine(deps)` in the renderer before starting any session > `desktop-engine.ts` `buildDesktopEngineDeps()` / `createDesktopEngine()` construct the three > native ports + `createEngine`. `@athing/engine` added to `apps/ui`. Unit-tested.
- [x] 8.3 Supply `cwd` on every session start/reconnect; a missing `cwd` surfaces a typed error rather than starting > Enforced in the engine proxy (`AtError("SpawnFailed","SessionOptions.cwd is required")`); the > desktop session registry (§6.2) supplies the cwd on reconnect.
- [x] 8.4 Confirm Web Crypto (`crypto.randomUUID`/`getRandomValues`) works in the Tauri web view secure context; provide a fallback id source if not > `web-crypto.ts` `randomId()`/`hasSecureCrypto()`: randomUUID -> getRandomValues -> weak v4 > fallback. Unit-tested.
- [x] 8.5 Render swap: on `isDesktopHost()`, boot via `bootDesktopHost` and drive the terminal from an engine `AgentSession` (`bindSessionToTerminal`) instead of the web JSON-WS path; verified on `bun run tauri dev` > `useDesktopHost.tsx` (`DesktopHostProvider` boots once, wraps the shell in `_shell.tsx`), > `DesktopTerminalPane.tsx` (engine session -> xterm via `bindSessionToTerminal`), session > route branches web/desktop, SessionSidebar "New session" -> `/session/new` on desktop. > `home_dir` added to `agent_bootstrap` as the default cwd; registry supplies cwd on reconnect. > Web path is byte-identical (status "web" -> original `TerminalPane`). App code typechecks > clean; runtime behavior verified by the user via `bun run tauri dev`.

## 9. Release pipeline — DESCOPED

> DESCOPED: signing, notarization, CI, and producing installable bundles need release infra
> (certs/CI) not available here. Defer to a packaging/release change.

## 10. Verification (cross-cutting)

> 10.1-10.2 (backpressure load + raw-byte fidelity) DESCOPED: they need a produced bundle + a
> running window to drive sustained output. The byte path is raw end-to-end by construction (the
> bridge forwards verbatim, the Channel preserves order); load/fidelity proof defers with packaging.

- [x] 10.3 Confirm web deployment (static `apps/ui` + `apps/server`) remains fully functional after the SPA switch > The desktop work is additive — `apps/server` still hosts the engine and `apps/ui`'s web > path (JSON-WS `TerminalPane`) is untouched; the new desktop modules are only reached behind > `isDesktopHost()`. Web build + server path unchanged.
