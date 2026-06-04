// Generic daemon + drive-plane conformance driver. Drives a daemon binary over
// its REAL control socket through the contract scenarios and asserts wire
// behavior — proving the Rust daemon is interchangeable with the reference
// without the engine distinguishing it.
//
// Usage: bun tests/benchmark/conformance.ts [binPath]
//   default binPath: packages/daemon-rs/target/release/athing-daemon

import { join } from "node:path";
import { existsSync } from "node:fs";
import { BenchClient } from "./client.ts";
import { launchDaemon } from "./daemon.ts";

const ROOT = join(import.meta.dir, "../..");
const binPath = process.argv[2] ?? join(ROOT, "packages/daemon-rs/target/release/athing-daemon");

let pass = 0;
let fail = 0;
function check(name: string, cond: boolean, detail = "") {
  if (cond) {
    pass++;
    console.log(`  ok  ${name}`);
  } else {
    fail++;
    console.log(`FAIL  ${name}  ${detail}`);
  }
}

function spawnMeta(id: string, command: string | undefined, args: string[], extra: object = {}) {
  return {
    type: "spawn",
    sessionId: id,
    command,
    args,
    token: "tok-" + id,
    cols: 100,
    rows: 30,
    cwd: "/tmp",
    ...extra,
  };
}

async function main() {
  if (!existsSync(binPath)) {
    console.error(
      `binary not found: ${binPath} (build with: cd daemon-rs && cargo build --release)`,
    );
    process.exit(2);
  }
  // Global watchdog so a stalled daemon can never hang the suite indefinitely.
  const watchdog = setTimeout(() => {
    console.error("conformance watchdog: a scenario stalled for 60s — aborting");
    process.exit(2);
  }, 60_000);
  watchdog.unref?.();
  const daemon = await launchDaemon(binPath, "conf");
  try {
    const c = new BenchClient(daemon.sockPath);
    await c.connect(); // handshake succeeds or connect() rejects
    check("hello handshake yields hello-ack", true);

    // Default launch is the login shell (no command).
    {
      const id = "shell";
      c.send(spawnMeta(id, undefined, []));
      const ack = await c.await("spawn-ack", (m) => m.sessionId === id);
      check(
        "default launch (no command) spawns login shell",
        (ack.meta.pid ?? 0) > 0,
        `pid=${ack.meta.pid}`,
      );
      c.send({ type: "kill", sessionId: id });
      await c.await("exit", (m) => m.sessionId === id);
    }

    // Explicit command output passes through unmodified.
    {
      const id = "echo";
      let data = "";
      const off = c.on((f) => {
        if (f.meta?.type === "data" && f.meta.sessionId === id && f.body) {
          data += Buffer.from(f.body).toString();
          c.send({ type: "ack", sessionId: id, bytes: f.body.length });
        }
      });
      c.send(spawnMeta(id, "/bin/sh", ["-c", "printf CONFORMANCE_OK"]));
      await c.await("spawn-ack", (m) => m.sessionId === id);
      await c.await("exit", (m) => m.sessionId === id);
      off();
      check(
        "explicit command output passes through verbatim",
        data.includes("CONFORMANCE_OK"),
        JSON.stringify(data),
      );
    }

    // Self-exit qualifiers.
    {
      const id = "ok";
      c.send(spawnMeta(id, "/bin/sh", ["-c", "exit 0"]));
      await c.await("spawn-ack", (m) => m.sessionId === id);
      const ex = await c.await("exit", (m) => m.sessionId === id);
      check("self-exit code 0 -> qualifier ok", ex.meta.qualifier === "ok", ex.meta.qualifier);
    }
    {
      const id = "err";
      c.send(spawnMeta(id, "/bin/sh", ["-c", "exit 3"]));
      await c.await("spawn-ack", (m) => m.sessionId === id);
      const ex = await c.await("exit", (m) => m.sessionId === id);
      check(
        "self-exit nonzero -> qualifier error",
        ex.meta.qualifier === "error",
        ex.meta.qualifier,
      );
      check(
        "exit raw carries diagnostic code",
        ex.meta.raw?.code === 3,
        JSON.stringify(ex.meta.raw),
      );
    }

    // Kill -> stopped-by-request.
    {
      const id = "killed";
      c.send(spawnMeta(id, "/bin/cat", []));
      await c.await("spawn-ack", (m) => m.sessionId === id);
      c.send({ type: "kill", sessionId: id });
      const ex = await c.await("exit", (m) => m.sessionId === id);
      check(
        "kill -> qualifier stopped-by-request",
        ex.meta.qualifier === "stopped-by-request",
        ex.meta.qualifier,
      );
    }

    // Input written verbatim, echoed by `cat`.
    {
      const id = "input";
      let echoed = "";
      const off = c.on((f) => {
        if (f.meta?.type === "data" && f.meta.sessionId === id && f.body) {
          echoed += Buffer.from(f.body).toString();
          c.send({ type: "ack", sessionId: id, bytes: f.body.length });
        }
      });
      c.send(spawnMeta(id, "/bin/cat", []));
      await c.await("spawn-ack", (m) => m.sessionId === id);
      c.send({ type: "input", sessionId: id }, new TextEncoder().encode("PING\n"));
      await Bun.sleep(300);
      off();
      check(
        "input bytes written verbatim (echoed by cat)",
        echoed.includes("PING"),
        JSON.stringify(echoed),
      );
      c.send({ type: "kill", sessionId: id });
      await c.await("exit", (m) => m.sessionId === id);
    }

    // Snapshot-capable subscribe yields a snapshot frame before data.
    {
      const id = "snap";
      c.send(spawnMeta(id, "/bin/sh", ["-c", "printf HELLO; sleep 5"]));
      await c.await("spawn-ack", (m) => m.sessionId === id);
      await Bun.sleep(200);
      const c2 = new BenchClient(daemon.sockPath, ["snapshot"]);
      await c2.connect();
      c2.send({ type: "subscribe", sessionId: id });
      const snap = await c2.await("snapshot", (m) => m.sessionId === id);
      check(
        "snapshot-capable subscribe yields snapshot frame",
        Array.isArray(snap.meta.cells) && typeof snap.meta.cursor?.x === "number",
      );
      c2.close();
      c.send({ type: "kill", sessionId: id });
      await c.await("exit", (m) => m.sessionId === id);
    }

    // Non-capable subscribe yields ring-buffer replay (data, not snapshot).
    {
      const id = "replay";
      c.send(spawnMeta(id, "/bin/sh", ["-c", "printf REPLAYDATA; sleep 5"]));
      await c.await("spawn-ack", (m) => m.sessionId === id);
      await Bun.sleep(200);
      const c3 = new BenchClient(daemon.sockPath, []); // no snapshot cap
      await c3.connect();
      c3.send({ type: "subscribe", sessionId: id });
      const d = await c3.await("data", (m) => m.sessionId === id);
      check(
        "non-capable subscribe yields ring-buffer replay",
        (d.body?.length ?? 0) > 0 && Buffer.from(d.body!).toString().includes("REPLAYDATA"),
      );
      c3.close();
      c.send({ type: "kill", sessionId: id });
      await c.await("exit", (m) => m.sessionId === id);
    }

    // list returns live session ids.
    {
      const id = "listed";
      c.send(spawnMeta(id, "/bin/cat", []));
      await c.await("spawn-ack", (m) => m.sessionId === id);
      const ids = await c.list();
      check("list returns live session ids", ids.includes(id), JSON.stringify(ids));
      c.send({ type: "kill", sessionId: id });
      await c.await("exit", (m) => m.sessionId === id);
    }

    // stop -> resume rejected with SessionStopped.
    {
      const id = "stopme";
      c.send(spawnMeta(id, "/bin/cat", []));
      await c.await("spawn-ack", (m) => m.sessionId === id);
      c.send({ type: "stop", sessionId: id });
      await Bun.sleep(300);
      c.send(spawnMeta("resumer", "/bin/cat", [], { resume: id }));
      const err = await c.await("error", (m) => m.sessionId === "resumer");
      check(
        "resume of stopped session rejected with SessionStopped",
        err.meta.code === "SessionStopped",
        err.meta.code,
      );
    }

    // Unresolvable command -> BinaryNotFound.
    {
      const id = "bad";
      c.send(spawnMeta(id, "definitely-not-real-binary-zzz", []));
      const err = await c.await("error", (m) => m.sessionId === id);
      check(
        "unresolvable command -> BinaryNotFound",
        err.meta.code === "BinaryNotFound",
        err.meta.code,
      );
    }

    c.close();
  } finally {
    daemon.stop();
  }

  console.log(`\n${pass} passed, ${fail} failed`);
  process.exit(fail === 0 ? 0 : 1);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
