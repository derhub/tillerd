import { expect, test } from "bun:test";

import type { TauriChannelLike, TauriCore } from "./tauri";

import { FILE_READ, FILE_SIZE } from "./file-source";
import { LIST_LOG_FILES, TauriLogSource } from "./log-source";

function fakeCore(
  handlers: Record<string, (args?: Record<string, unknown>) => unknown>,
): TauriCore {
  return {
    invoke: (async (cmd: string, args?: Record<string, unknown>) =>
      handlers[cmd]?.(args)) as TauriCore["invoke"],
    createChannel: (): TauriChannelLike => ({ onmessage: null }),
    listen: async () => () => {},
  };
}

test("list returns the host's log files with sizes", async () => {
  const entries = [
    {
      name: "tillerd-daemon.2026-06-13.log",
      path: "/r/logs/tillerd-daemon.2026-06-13.log",
      size: 12,
    },
    { name: "tillerd-gate.2026-06-13.log", path: "/r/logs/tillerd-gate.2026-06-13.log", size: 4 },
  ];
  const src = new TauriLogSource(fakeCore({ [LIST_LOG_FILES]: () => entries }));
  expect(await src.list()).toEqual(entries);
});

test("size delegates to file_size and preserves null for an absent file", async () => {
  const src = new TauriLogSource(fakeCore({ [FILE_SIZE]: () => null }));
  expect(await src.size("/r/logs/absent.log")).toBeNull();
});

test("read returns the bytes the host yields, short at end of file", async () => {
  const src = new TauriLogSource(
    fakeCore({ [FILE_READ]: () => new Uint8Array([104, 105]) }), // "hi", fewer than requested
  );
  const bytes = await src.read("/r/logs/x.log", 0, 100);
  expect(Array.from(bytes)).toEqual([104, 105]);
});
