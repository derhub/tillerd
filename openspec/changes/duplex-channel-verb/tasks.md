## 1. transport_channel! macro + tagged send (specs: session lifecycle, off-telemetry send) (D1, D2)

- [x] 1.1 (red) Tests: a tagged data send writes to the runtime AND produces no `command`/`query` span (off-telemetry — the keystroke-never-logged invariant); a `Close` send tears down (no further frames); `Resize` reaches the runtime.
- [x] 1.2 (green) Add `transport_channel!` generating two shims: open (`ipc::Channel` + params -> build `ChannelSink`, register via the open command, return key) and `name_send(key, msg)` where `msg` is `Input(bytes)|Resize{cols,rows}|Close`. `Input`/`Resize` call the off-telemetry runtime path on `cx` (not `bus.execute`); `Close` -> `bus.execute(UnsubscribeSurface)`. `Input` via Tauri Raw Request (raw bytes).

## 2. Migrate surface onto one channel (specs: surface over one channel) (D3, D4)

- [x] 2.1 (green) `surface_channel` open is defined via `transport_channel!`: spawns, queries placement, registers `ChannelSink` via `SubscribeSurface`, attaches. Legacy `surface_create`/`surface_input`/`surface_resize` shims kept (Phase-1 UI still calls them). Gap vs `surface_create`: channel open always spawns fresh -- no revisit/scrollback-replay path. Noted; out of scope.
- [x] 2.2 (red->green) Parity tests added in `transport/channel.rs`: `output_frames_reach_the_registered_sink` (subscribe `ByteRecorder` sink via `SubscribeSurface`, dispatch via `surface_sinks().dispatch`, assert frame arrives); `two_inputs_arrive_in_order` (two sequential `Input` sends arrive at probe in order). Pre-existing tests cover: input->runtime bytes, off-telemetry invariant, resize->runtime, `Close`->unsubscribes-and-stops-frames. 537 tests pass; clippy clean.

## 3. Duplex client binding (D1)

- [x] 3.1 (green) Add `openChannel(name, params) -> { onmessage, send, resize, close }` in `@tillerd/client-bindings`: creates the receive `Channel`, invokes open, exposes `send`/`resize`/`close` over `name_send`. Generalize/retire `makeSurfaceChannel`/`makeStreamChannel` as appropriate (additive; keep working). Type-check the package.

## 4. Verify + fix-all gate

- [x] 4.1 Backend gate green: `cargo test -p tillerd-orchestrator -p tillerd-desktop` = 537 pass (off-telemetry send, surface parity, teardown), clippy `--workspace -D warnings` clean, `sg scan` only the pre-existing client-bindings TS comment, `client-bindings` tsc clean. `bun run verify`/e2e deferred (Phase-1 UI).
- [x] 4.2 Confirmed `docs/adr/0043` matches the shipped shape.

## Staged for Phase-1 (surface cutover — coupled to the UI migration)

- `transport_channel!` macro + `SurfaceClientMsg` tagged send + off-telemetry invariant + `openSurfaceChannel` duplex handle: DELIVERED + tested.
- `surface_channel` is a backend instantiation (registered + contract-tested) but NOT yet in the specta `collect_commands!`/typed bindings; `openSurfaceChannel` currently routes to the legacy `surfaceCreate`/`surfaceInput`/`surfaceResize`/`surfaceClose` commands (full behavior incl. revisit/scrollback — correct today).
- Remaining cutover (Phase-1, with the renderer adopting `openSurfaceChannel`): expose `surface_channel`/`surface_channel_send_cmd` in specta; fill `surface_channel` open's revisit + scrollback-replay parity with `surface_create`; repoint `openSurfaceChannel` to the channel commands; retire the legacy surface shims.
