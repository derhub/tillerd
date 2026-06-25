import { encodeFrame } from "@tillerd/sdk";
import { test, expect, describe, mock } from "bun:test";

import { TauriAppData } from "./app-data";
import { TauriLogger } from "./logger";
import { TauriDaemonTransport, type TauriCore, type TauriChannelLike } from "./tauri";

type Call = { cmd: string; args?: Record<string, unknown> };

class FakeCore implements TauriCore {
  readonly calls: Call[] = [];
  channel: TauriChannelLike | null = null;
  private responders = new Map<string, (args?: Record<string, unknown>) => unknown>();

  on(cmd: string, fn: (args?: Record<string, unknown>) => unknown): void {
    this.responders.set(cmd, fn);
  }
  async invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    this.calls.push({ cmd, args });
    const r = this.responders.get(cmd);
    return (r ? r(args) : undefined) as T;
  }
  createChannel(): TauriChannelLike {
    this.channel = { onmessage: null };
    return this.channel;
  }
  async listen(): Promise<() => void> {
    return () => {};
  }
  deliver(meta: object, body?: Uint8Array): void {
    const buf = encodeFrame(meta, body);
    this.channel?.onmessage?.(buf);
  }
}

describe("TauriDaemonTransport", () => {
  test("connect: opens a channel, invokes daemon_connect, sends hello, resolves on hello-ack", async () => {
    const core = new FakeCore();
    const t = new TauriDaemonTransport(core);
    const connected = t.connect();
    await Promise.resolve();
    expect(core.calls[0]!.cmd).toBe("daemon_connect");
    expect(core.channel).not.toBeNull();
    await Promise.resolve();
    const sent = core.calls.find((c) => c.cmd === "daemon_send");
    expect(sent).toBeDefined();
    core.deliver({ type: "hello-ack", version: 1, daemonVersion: "0.0.1" });
    await connected;
  });

  test("dispatches inbound session frames over the channel with raw body", async () => {
    const core = new FakeCore();
    const t = new TauriDaemonTransport(core);
    const connected = t.connect();
    await Promise.resolve();
    await Promise.resolve();
    core.deliver({ type: "hello-ack", version: 1, daemonVersion: "0.0.1" });
    await connected;

    const got: Uint8Array[] = [];
    t.subscribe("s1", (_f, body) => body && got.push(body));
    core.deliver({ type: "data", sessionId: "s1", bodyLen: 3 }, new Uint8Array([7, 8, 9]));
    expect([...got[0]!]).toEqual([7, 8, 9]);
  });

  test("outbound send invokes daemon_send with raw bytes", async () => {
    const core = new FakeCore();
    const t = new TauriDaemonTransport(core);
    const connected = t.connect();
    await Promise.resolve();
    await Promise.resolve();
    core.deliver({ type: "hello-ack", version: 1, daemonVersion: "0.0.1" });
    await connected;

    core.calls.length = 0;
    t.send({ type: "list" });
    const sent = core.calls.find((c) => c.cmd === "daemon_send");
    expect(Array.isArray(sent!.args!.bytes)).toBe(true);
  });
});

describe("TauriLogger", () => {
  test("forwards each level to the core", async () => {
    const core = new FakeCore();
    const log = new TauriLogger(core);
    const spy = mock(() => {});
    const orig = console.info;
    console.info = spy;
    try {
      log.info("hi", { a: 1 });
    } finally {
      console.info = orig;
    }
    await Promise.resolve();
    const fwd = core.calls.find((c) => c.cmd === "log_forward");
    expect(fwd!.args).toMatchObject({ level: "info", msg: "hi", extra: { a: 1 } });
  });

  test("without a core it still logs and never throws", () => {
    const log = new TauriLogger();
    expect(() => log.error("boom")).not.toThrow();
  });
});

describe("TauriAppData", () => {
  test("reconcile removes only entries whose session is no longer live", async () => {
    const core = new FakeCore();
    core.on("registry_list", () => [
      { sessionId: "live", cwd: "/a" },
      { sessionId: "stale", cwd: "/b" },
    ]);
    const removed: string[] = [];
    core.on("registry_remove", (a) => {
      removed.push(a!.sessionId as string);
    });
    const data = new TauriAppData(core);
    await data.reconcile(["live"]);
    expect(removed).toEqual(["stale"]);
  });
});
