# a-thing

Experimental toolkit for driving coding agents.

Early and unstable — still exploring the design. No stable API, no roadmap yet.
Expect things to change or break.

## Requirements

- [Bun](https://bun.com) 1.3.14
- Rust toolchain (the PTY daemon builds via `cargo`)
- macOS or Linux

## Develop

```bash
bun install     # install workspace dependencies
bun run dev     # boot the dev stack (server + UI)
bun run verify  # format, types, lint, tests, e2e — run before pushing
```

## Layout

```
apps/        server, ui, cli, desktop, and services
packages/    sdk, engine, adapters, and supporting libraries
tests/       integration and e2e suites
```

## Documentation

No docs site yet — see [`docs/`](./docs) for architecture decision records and guides.

## License

Released under the [MIT License](LICENSE).
