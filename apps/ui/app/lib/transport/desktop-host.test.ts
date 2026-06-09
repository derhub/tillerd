import { test, expect, describe } from "bun:test";
import { encodeFrame, FrameDecoder, parseDaemonFrame } from "@athing/sdk";
import { bootDesktopHost } from "./desktop-host";
import type { TauriCore, TauriChannelLike } from "./tauri";

type Call = { cmd: string; args?: Record<string, unknown> };

/**
 * A fake native core that also plays the daemon byte-bridge: it answers the handshake with a
 * hello-ack over the channel and a list-ack for `list`, so `transport.connect()` and
 * `engine.listSessions()` resolve.
 */
function makeCore(version: string, liveIds: string[] = []): { core: TauriCore; calls: Call[] } {
  const calls: Call[] = [];
  let channel: TauriChannelLike | null = null;
  const outbound = new FrameDecoder();
  const deliver = (meta: object) => channel?.onmessage?.(encodeFrame(meta));

  const core: TauriCore = {
    async invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T> {
      calls.push({ cmd, args });
      switch (cmd) {
        case "agent_bootstrap":
          return {
            path: "/usr/bin/claude",
            version,
            hookCommand: "/x/bin/athing-notify",
            athingDir: "/x/athing",
            agentHome: "/home/u/.claude",
            homeDir: "/home/u",
          } as T;
        case "daemon_ensure":
          return { ownership: "owned", socket: "/x/daemon.sock" } as T;
        case "daemon_connect":
          queueMicrotask(() => deliver({ type: "hello-ack", version: 1, daemonVersion: "0.0.1" }));
          return undefined as T;
        case "daemon_send": {
          const bytes = new Uint8Array(args!.bytes as number[]);
          for (const { meta } of outbound.push(bytes)) {
            if (parseDaemonFrame(meta)?.type === "list") {
              queueMicrotask(() => deliver({ type: "list-ack", ids: liveIds }));
            }
          }
          return undefined as T;
        }
        case "registry_list":
          return liveIds.map((id) => ({ sessionId: id, cwd: "/w" })) as T;
        default:
          return undefined as T; // registry_remove, etc.
      }
    },
    createChannel(): TauriChannelLike {
      channel = { onmessage: null };
      return channel;
    },
  };
  return { core, calls };
}

describe("bootDesktopHost", () => {
  test("resolves agent, ensures daemon, connects transport, reconciles, builds engine", async () => {
    const { core, calls } = makeCore("1.5.0", ["stale"]);
    const host = await bootDesktopHost(core);

    expect(host.agent.name).toBe("claude-code");
    expect(typeof host.engine.start).toBe("function");
    expect(calls.map((c) => c.cmd)).toContain("daemon_connect");
    // reconcile ran: the live id matched, so nothing was removed; registry_list was queried.
    expect(calls.some((c) => c.cmd === "registry_list")).toBe(true);
  });

  test("rejects an unsupported agent version before ensuring the daemon", async () => {
    const { core, calls } = makeCore("0.1.0");
    await expect(bootDesktopHost(core)).rejects.toMatchObject({ kind: "VersionUnsupported" });
    expect(calls.some((c) => c.cmd === "daemon_ensure")).toBe(false);
  });
});
