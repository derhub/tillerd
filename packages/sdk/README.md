# @athing/sdk

Ports and types only — zero deps, zero implementation.

## Public API

```ts
import type {
  AgentSession,
  Engine,
  AgentDefinition,
  SessionOptions,
  HookEvent,
  HookEventType,
  SessionStatus,
  ContentEvent,
  ExitEvent,
} from "@athing/sdk";
import { AtError } from "@athing/sdk";
```

### `Engine`

```ts
interface Engine {
  start(adapter: AgentDefinition, options?: SessionOptions): Promise<AgentSession>;
  shutdown(): Promise<void>;
}
```

Create via `createEngine()` from `@athing/engine`. One engine hosts many concurrent sessions.

### `AgentSession`

```ts
session.send(text); // gated on IDLE — queued until ready
session.input(bytes); // raw bytes, no gating
session.interrupt(); // cancel current turn, keep session alive
session.resize(cols, rows);
session.kill(); // → Promise<ExitEvent>

session.onData(handler); // → unsubscribe fn
session.onStatus(handler);
session.onContent(handler);
session.onError(handler);
session.onExit(handler);
```

### Status model

| Status          | Meaning                         |
| --------------- | ------------------------------- |
| `IDLE`          | Ready for the next prompt       |
| `WORKING`       | Turn in progress                |
| `WAITING_INPUT` | Agent is waiting for user input |
| `DONE`          | Session ended                   |

### Error taxonomy

All errors are `AtError` instances with a `kind` discriminant:

`BinaryNotFound` · `NotAuthenticated` · `SpawnFailed` · `HookInstallFailed` ·
`TranscriptUnavailable` · `TransportClosed` · `Timeout` · `VersionUnsupported`

## Writing an adapter

An `AgentDefinition` is config data + three parse functions:

```ts
const myAgent: AgentDefinition = {
  name: "my-agent",
  launch: { command: "my-cli", args: ["--session-id", "{id}"], flags: [] },
  hookInstall: {
    settingsPath: "~/.myagent/settings.json",
    notifyScriptPath: "bin/athing-notify",
    events: ["SessionStart", "Stop"],
  },
  cliVersionRange: ">=2.0.0",
  parseHook(raw) {
    /* raw payload → HookEvent */
  },
  transcriptPath(sessionId, cwd) {
    /* → path string */
  },
  parseTranscriptEntry(line) {
    /* → ContentEvent | null */
  },
};
```

The engine calls your parse functions; it never imports your adapter directly.
