---
description: Use Bun instead of Node.js, npm, pnpm, or vite.
globs: "*.ts, *.tsx, *.html, *.css, *.js, *.jsx, package.json"
alwaysApply: false
---

Default to using Bun instead of Node.js.

- Use `bun <file>` instead of `node <file>` or `ts-node <file>`
- Use `bun test` instead of `jest` or `vitest`
- Use `bun build <file.html|file.ts|file.css>` instead of `webpack` or `esbuild`
- Use `bun install` instead of `npm install` or `yarn install` or `pnpm install`
- Use `bun run <script>` instead of `npm run <script>` or `yarn run <script>` or `pnpm run <script>`
- Use `bunx <package> <command>` instead of `npx <package> <command>`
- Bun automatically loads .env, so don't use dotenv.

## Project: a-thing

An SDK that drives coding-agent CLIs (Claude Code first) so any UI can integrate them without
reimplementing the agent loop or holding credentials. Active design in `openspec/`; durable
architectural decisions in `docs/adr/`.

### Tech stack

- Bun runtime, TypeScript, turbo monorepo.
- valibot — standard validation library (adapter config, HookEvents, wire messages).
- node-pty v1.1.0 (pinned) — PTY binding, confined to the engine.
- React + react-router + @xterm/xterm — `apps/ui`.

### Packages (ports-and-adapters; deps point inward to sdk)

- `@athing/sdk` — contracts, types, and pure contract functions (e.g. wire codec, exit-qualifier and signal mapping, snapshot rendering). Zero I/O, zero side effects, zero mutable module state; no adapter or transport deps. A function belongs here only if it is a deterministic, pure operation over the contract types.
- `@athing/engine` — the machinery; depends on sdk; never imports a specific adapter.
- `@athing/adapter-claude-code` — hybrid AgentDefinition (config data + parse functions).
- `apps/server` — composition root; injects the adapter into the engine; WebSocket + HTTP.
- `apps/ui` — react-router SPA; depends on sdk types + network.

### Architecture

- Drive the agent via an interactive PTY on the user's own login — no API key, no vendor SDK.
- The engine consumes lifecycle only as the `HookEvent` contract; a generic loopback hook
  ingress (per-session token) calls `adapter.parseHook`.
- Status = generic contract-enum -> {IDLE|WORKING|WAITING_INPUT|DONE}; content = transcript
  read-on-hook; adapters are hybrid (data + parse functions).
- Raw bytes end-to-end (no ANSI stripping / no UTF-8 re-decode). Platform: macOS/Linux for v1.
- Honor the reliability contract and all decisions in `docs/adr/0001-0007`.

### Constraints & conventions

- One subscription = one user (bring-your-own-login); multi-user/commercial needs API keys
  under the Commercial Terms (out of scope).
- Conventional Commits & Branches. Do not use the word "Claude" or any emojis in commit
  messages or docs.

## Testing

Use `bun test` to run tests.

```ts#index.test.ts
import { test, expect } from "bun:test";

test("hello world", () => {
  expect(1).toBe(1);
});
```

## Frontend

Use HTML imports with `Bun.serve()`. Don't use `vite`. HTML imports fully support React, CSS, Tailwind.
