# tillerd

A local-first desktop workbench for coding agents — a Tauri app over a Rust orchestrator
that runs terminal surfaces, groups them into sessions / projects / workspaces, and persists
everything to a local `tillerd.db`.

Pre-1.0 and unstable; APIs may break between minor versions. See [`ROADMAP.md`](./ROADMAP.md)
for the plan and [`CHANGELOG.md`](./CHANGELOG.md) for what has shipped. The working app landed
at 0.0.20; 0.x is deliberately terminal-only (the agent surface returns in 1.0.0 — ADR-0027).

## Requirements

- [Bun](https://bun.com) 1.3.14
- Rust toolchain (the orchestrator, daemon, and services build via `cargo`)
- macOS or Linux

## Develop

```bash
bun install     # install workspace dependencies
bun run dev      # boot the dev stack (turbo)
bun run verify  # format, types, lint, tests, e2e — run before pushing
```

Rust dev loop: `cargo nextest run` for tests, `cargo check` for fast type feedback.

## Layout

```
apps/        desktop (Tauri host), ui, orchestrator services (daemon-pty, gate,
             gate-notify, mcp-gateway, memorya), server, cli
crates/      orchestrator (entities / infra / shared / app / boot) + Rust libraries
packages/    client-bindings (generated TS types)
tests/       desktop-e2e and integration suites
```

## Documentation

No docs site yet — see [`docs/`](./docs) for architecture decision records and guides.

## License

Released under the [MIT License](LICENSE).
