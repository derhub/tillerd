// End-to-end upgrade handoff: spawn a session, trigger `upgrade`, confirm the
// successor adopts it (same socket) and the session is still alive (its cat
// process keeps echoing after the swap).

import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { BenchClient } from "./client.ts";

const bin = process.argv[2] ?? join(import.meta.dir, "../../packages/daemon-rs/target/release/athing-daemon");
const dir = mkdtempSync(join(tmpdir(), "athing-upgrade-"));
const sock = join(dir, "daemon.sock");
const manifestPath = join(dir, "daemon.json");

function manifestPid(): number | null {
  try { return JSON.parse(readFileSync(manifestPath, "utf8")).pid; } catch { return null; }
}

async function echoOnce(c: BenchClient, id: string, marker: string, timeoutMs = 2500): Promise<boolean> {
  const p = new Promise<boolean>((res) => {
    const off = c.on((f) => {
      if (f.meta?.type === "data" && f.meta.sessionId === id && f.body && Buffer.from(f.body).includes(marker)) {
        off(); res(true);
      }
    });
    setTimeout(() => { off(); res(false); }, timeoutMs);
  });
  c.send({ type: "input", sessionId: id }, new TextEncoder().encode(marker + "\n"));
  return p;
}

let fail = 0;
function check(name: string, ok: boolean) { console.log(`${ok ? "ok  " : "FAIL"} ${name}`); if (!ok) fail++; }

// Launch the predecessor daemon directly (we manage the handoff ourselves).
const proc = Bun.spawn([bin], { env: { ...process.env, ATHING_DIR: dir }, stdio: ["ignore", "inherit", "inherit"] });
{
  const dl = Date.now() + 15000;
  while (Date.now() < dl && !existsSync(sock)) await Bun.sleep(50);
  await Bun.sleep(150);
}
const oldPid = manifestPid();

const c1 = new BenchClient(sock);
await c1.connect();
c1.on((f) => { if (f.meta?.type === "data" && f.meta.sessionId === "surv" && f.body) c1.send({ type: "ack", sessionId: "surv", bytes: f.body.length }); });
c1.send({ type: "spawn", sessionId: "surv", command: "/bin/cat", args: [], token: "t", cols: 80, rows: 24, cwd: "/tmp" });
const ack = await c1.await("spawn-ack", (m) => m.sessionId === "surv");
check("session spawned", (ack.meta.pid ?? 0) > 0);
await Bun.sleep(150);
check("echoes BEFORE upgrade", await echoOnce(c1, "surv", "BEFORE_UPGRADE"));

// Trigger the handoff.
c1.send({ type: "upgrade" });

// Wait for the successor to take over the manifest (new pid).
let newPid: number | null = null;
{
  const dl = Date.now() + 15000;
  while (Date.now() < dl) {
    const p = manifestPid();
    if (p !== null && p !== oldPid) { newPid = p; break; }
    await Bun.sleep(100);
  }
}
check("successor took over (manifest pid changed)", newPid !== null && newPid !== oldPid);
c1.close();
await Bun.sleep(300);

// Reconnect to the SAME socket — now served by the successor.
const c2 = new BenchClient(sock);
await c2.connect();
c2.on((f) => { if (f.meta?.type === "data" && f.meta.sessionId === "surv" && f.body) c2.send({ type: "ack", sessionId: "surv", bytes: f.body.length }); });
const ids = await c2.list();
check("session present after handoff", ids.includes("surv"));
c2.send({ type: "subscribe", sessionId: "surv" });
await Bun.sleep(150);
check("session ALIVE after handoff (echoes AFTER)", await echoOnce(c2, "surv", "AFTER_UPGRADE"));

c2.close();
// Cleanup: kill old (likely dead) and successor.
try { proc.kill("SIGKILL"); } catch {}
if (newPid) { try { process.kill(newPid, "SIGKILL"); } catch {} }
if (oldPid) { try { process.kill(oldPid, "SIGKILL"); } catch {} }
rmSync(dir, { recursive: true, force: true });

console.log(`\n${fail === 0 ? "PASS" : "FAIL"} — ${fail} failure(s)`);
process.exit(fail === 0 ? 0 : 1);
