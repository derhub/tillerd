# Quick start

Get the dev stack running locally.

## Prerequisites

- [Bun](https://bun.com) 1.3.14
- A Rust toolchain (`cargo`) — the PTY daemon is built from source
- macOS or Linux
- An installed, authenticated coding-agent CLI available on your `PATH`

## Install

```bash
git clone <repo-url>
cd tillerd
bun install
```

## Build

```bash
bun run build   # builds all packages, including the Rust PTY daemon
```

## Run

```bash
bun run dev     # boots the server + UI dev stack via turbo
```

## Verify

Run the full battery before pushing — format, types, lint, tests, and the
self-provisioning daemon/engine e2e:

```bash
bun run verify
```

Next: see the [project README](../../README.md) for architecture and layout, and
the [docs index](../README.md) for guides and decision records.
