import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { notifyCommand, notifyScriptPath, prepareNotifyScript } from "../src/ingress";

const NOTIFY_BIN = path.join(import.meta.dir, "../../../bin/athing-notify");

interface Received {
  body: string;
  token: string | null;
  sessionId: string | null;
  contentType: string | null;
}

function shortSockPath(): string {
  // macOS caps unix socket paths near 104 chars — keep it short.
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "ath-"));
  return path.join(dir, "h.sock");
}

async function runNotify(env: Record<string, string>, payload: string): Promise<number> {
  const proc = Bun.spawn([NOTIFY_BIN], {
    stdin: new TextEncoder().encode(payload),
    stdout: "ignore",
    stderr: "ignore",
    env: { ...process.env, ...env },
  });
  return await proc.exited;
}

describe("bin/athing-notify", () => {
  const cleanups: Array<() => void> = [];
  afterEach(() => {
    for (const c of cleanups.splice(0)) c();
  });

  test("forwards payload and auth headers over the unix socket", async () => {
    const sock = shortSockPath();
    const got = Promise.withResolvers<Received>();
    const server = Bun.serve({
      unix: sock,
      async fetch(req) {
        got.resolve({
          body: await req.text(),
          token: req.headers.get("x-session-token"),
          sessionId: req.headers.get("x-session-id"),
          contentType: req.headers.get("content-type"),
        });
        return new Response("ok");
      },
    });
    cleanups.push(() => server.stop(true));

    const payload = JSON.stringify({ hook_event_name: "UserPromptSubmit", session_id: "s1" });
    const code = await runNotify(
      {
        ATHING_BRIDGE_URL: sock,
        ATHING_SESSION_TOKEN: "tok-123",
        ATHING_SESSION_ID: "s1",
      },
      payload,
    );

    const received = await got.promise;
    expect(code).toBe(0);
    expect(received.body).toBe(payload);
    expect(received.token).toBe("tok-123");
    expect(received.sessionId).toBe("s1");
    expect(received.contentType).toBe("application/json");
  });

  test("exits 0 and delivers nothing when bridge url is absent", async () => {
    const code = await runNotify({ ATHING_BRIDGE_URL: "" }, "{}");
    expect(code).toBe(0);
  });

  test("exits 0 when the endpoint is dead (fire-and-forget)", async () => {
    const sock = shortSockPath(); // path exists in tmp dir, but no server listening
    const code = await runNotify(
      { ATHING_BRIDGE_URL: sock, ATHING_SESSION_TOKEN: "t", ATHING_SESSION_ID: "s" },
      "{}",
    );
    expect(code).toBe(0);
  });
});

describe("notify client resolution", () => {
  const prev = process.env["ATHING_NOTIFY_BIN"];
  afterEach(() => {
    if (prev === undefined) delete process.env["ATHING_NOTIFY_BIN"];
    else process.env["ATHING_NOTIFY_BIN"] = prev;
  });

  test("notifyCommand resolves the committed bin path", () => {
    process.env["ATHING_NOTIFY_BIN"] = NOTIFY_BIN;
    expect(notifyCommand()).toBe(path.resolve(NOTIFY_BIN));
    expect(notifyScriptPath()).toBe(path.resolve(NOTIFY_BIN));
  });

  test("prepareNotifyScript returns the executable command", () => {
    process.env["ATHING_NOTIFY_BIN"] = NOTIFY_BIN;
    expect(prepareNotifyScript()).toEqual({ command: path.resolve(NOTIFY_BIN), updated: false });
  });

  test("prepareNotifyScript throws HookInstallFailed when the client is absent", () => {
    const missing = path.join(os.tmpdir(), "athing-nonexistent", "athing-notify");
    expect(() => prepareNotifyScript(missing)).toThrow(/not found/);
  });
});
