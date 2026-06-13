# @tillerd/sdk

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
} from "@tillerd/sdk";
import { AtError } from "@tillerd/sdk";
```

### `Engine`

```ts
interface Engine {
  start(adapter: AgentDefinition, options?: SessionOptions): Promise<AgentSession>;
  shutdown(): Promise<void>;
}
```

Create via `createEngine()` from `@tillerd/engine`. One engine hosts many concurrent sessions.

### `AgentSession`

```ts
session.send(text); // gated on IDLE — queued until ready
session.input(bytes); // raw bytes, no gating
session.interrupt(); // cancel current turn, keep session alive
session.resize(cols, rows);
session.kill(); // -> Promise<ExitEvent>

session.onData(handler); // -> unsubscribe fn
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

An `AgentDefinition` is config data + three parse functions, with zero host I/O so it
is import-safe in any runtime:

```ts
const myAgent: AgentDefinition = {
  name: "my-agent",
  launch: { command: "my-cli", args: ["--session-id", "{id}"], flags: [] },
  interruptSequence: "\x1b",
  cliVersionRange: ">=2.0.0",
  binaryResolution: {
    overrideEnvVar: "MY_AGENT_EXECUTABLE",
    binaryName: "my-cli",
    commonLocations: ["/usr/local/bin/my-cli", "~/.local/bin/my-cli"],
  },
  parseHook(raw) {
    /* raw payload -> HookEvent */
  },
  transcriptPath(sessionId, cwd, agentHome) {
    /* -> path string, assembled from agentHome with pure string ops */
  },
  parseTranscriptEntry(line) {
    /* -> ContentEvent | null */
  },
};
```

The engine calls your parse functions; it never imports your adapter directly. Host
setup (e.g. installing hooks) is a separate `defineSetup({ install, uninstall })` export
the installer invokes with a `SetupContext` — the definition itself stays I/O-free.
