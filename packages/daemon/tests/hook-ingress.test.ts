import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { HookIngress } from "../src/hook-ingress";
import { createLogger } from "@athing/logger";

function tmpSockPath(): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "athing-hook-"));
  return path.join(dir, "hooks.sock");
}

async function post(
  sockPath: string,
  headers: Record<string, string>,
  body: unknown,
): Promise<Response> {
  return fetch("http://localhost/hook", {
    unix: sockPath,
    method: "POST",
    headers: { "content-type": "application/json", ...headers },
    body: JSON.stringify(body),
  } as RequestInit & { unix: string });
}

describe("HookIngress", () => {
  const ingresses: HookIngress[] = [];
  const sockDirs: string[] = [];

  afterEach(() => {
    for (const i of ingresses.splice(0)) i.stop();
    for (const d of sockDirs.splice(0)) {
      try {
        fs.rmSync(d, { recursive: true, force: true });
      } catch {}
    }
  });

  function makeIngress(
    tokens: Record<string, string>,
    onHook: (id: string, payload: unknown) => void = () => {},
  ): string {
    const sockPath = tmpSockPath();
    sockDirs.push(path.dirname(sockPath));
    const ingress = new HookIngress({
      socketPath: sockPath,
      getToken: (sessionId) => tokens[sessionId] ?? null,
      onHook,
      logger: createLogger("test"),
    });
    ingress.start();
    ingresses.push(ingress);
    return sockPath;
  }

  test("missing session-id header returns 401", async () => {
    const sock = makeIngress({});
    const res = await post(sock, { "x-session-token": "tok" }, {});
    expect(res.status).toBe(401);
  });

  test("missing token header returns 401", async () => {
    const sock = makeIngress({});
    const res = await post(sock, { "x-session-id": "s1" }, {});
    expect(res.status).toBe(401);
  });

  test("unknown session returns 403", async () => {
    const sock = makeIngress({});
    const res = await post(sock, { "x-session-id": "unknown", "x-session-token": "tok" }, {});
    expect(res.status).toBe(403);
  });

  test("wrong token returns 403", async () => {
    const sock = makeIngress({ s1: "correct" });
    const res = await post(sock, { "x-session-id": "s1", "x-session-token": "wrong" }, {});
    expect(res.status).toBe(403);
  });

  test("valid request returns 200 and calls onHook", async () => {
    const calls: [string, unknown][] = [];
    const sock = makeIngress({ s1: "tok" }, (id, payload) => calls.push([id, payload]));
    const res = await post(sock, { "x-session-id": "s1", "x-session-token": "tok" }, { event: "stop" });
    expect(res.status).toBe(200);
    expect(calls).toHaveLength(1);
    expect(calls[0]).toEqual(["s1", { event: "stop" }]);
  });

  test("duplicate payload is idempotent — onHook called only once", async () => {
    const calls: unknown[] = [];
    const sock = makeIngress({ s1: "tok" }, (_, p) => calls.push(p));
    const payload = { event: "stop" };
    await post(sock, { "x-session-id": "s1", "x-session-token": "tok" }, payload);
    await post(sock, { "x-session-id": "s1", "x-session-token": "tok" }, payload);
    expect(calls).toHaveLength(1);
  });

  test("GET request returns 405", async () => {
    const sock = makeIngress({});
    const res = await fetch("http://localhost/", {
      unix: sock,
    } as RequestInit & { unix: string });
    expect(res.status).toBe(405);
  });
});
