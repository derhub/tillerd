# Changelog

All notable changes to this project are recorded here. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); this project is pre-1.0
and APIs may break between minor versions.

## [Unreleased]

Pre-release (0.0.x). Core components are scaffolded but the app does not yet work
end-to-end. The first working release will be tagged 0.1.0.

### Added

- **PTY daemon** — detached, multi-session terminal owner with binary-framed IPC,
  session persistence, and crash recovery.
- **Gate** — single multiplexed socket fronting all agent-facing traffic; routes by
  preamble (hook, tool, subscribe, admin, mcp) with auth, normalization, and
  fan-out, and is the observability chokepoint.
- **Desktop shell** — Tauri app that orchestrates sessions (mint, register,
  adopt-or-spawn) and hosts the engine in the renderer.
- **Shared libraries** — `contracts` (wire types + frame codec), `service-host`,
  `process-launch`, `gate-client`, `redact`.
- **Observability** — correlation-bound log context and resource identity
  (OTel-ready).

[Unreleased]: https://github.com/derhub/tillerd
