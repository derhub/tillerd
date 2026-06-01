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

- `@athing/sdk` — contracts/types only, zero deps, zero impl.
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

## APIs

- `Bun.serve()` supports WebSockets, HTTPS, and routes. Don't use `express`.
- `bun:sqlite` for SQLite. Don't use `better-sqlite3`.
- `Bun.redis` for Redis. Don't use `ioredis`.
- `Bun.sql` for Postgres. Don't use `pg` or `postgres.js`.
- `WebSocket` is built-in. Don't use `ws`.
- Prefer `Bun.file` over `node:fs`'s readFile/writeFile
- Bun.$`ls` instead of execa.

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

Server:

```ts#index.ts
import index from "./index.html"

Bun.serve({
  routes: {
    "/": index,
    "/api/users/:id": {
      GET: (req) => {
        return new Response(JSON.stringify({ id: req.params.id }));
      },
    },
  },
  // optional websocket support
  websocket: {
    open: (ws) => {
      ws.send("Hello, world!");
    },
    message: (ws, message) => {
      ws.send(message);
    },
    close: (ws) => {
      // handle close
    }
  },
  development: {
    hmr: true,
    console: true,
  }
})
```

HTML files can import .tsx, .jsx or .js files directly and Bun's bundler will transpile & bundle automatically. `<link>` tags can point to stylesheets and Bun's CSS bundler will bundle.

```html#index.html
<html>
  <body>
    <h1>Hello, world!</h1>
    <script type="module" src="./frontend.tsx"></script>
  </body>
</html>
```

With the following `frontend.tsx`:

```tsx#frontend.tsx
import React from "react";
import { createRoot } from "react-dom/client";

// import .css files directly and it works
import './index.css';

const root = createRoot(document.body);

export default function Frontend() {
  return <h1>Hello, world!</h1>;
}

root.render(<Frontend />);
```

Then, run index.ts

```sh
bun --hot ./index.ts
```

For more information, read the Bun API docs in `node_modules/bun-types/docs/**.mdx`.
