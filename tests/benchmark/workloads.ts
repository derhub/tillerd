// Fixed, reproducible workloads exercising the daemon hot paths. Both daemons
// are driven with the SAME explicit command so the comparison isolates daemon
// overhead (framing, fan-out, VT parse) rather than launch differences.

import { BenchClient } from "./client.ts";
import { now, rssKb, summarize } from "./metrics.ts";

// Fixed inputs — identical across daemons and runs.
export const PARAMS = {
  spawnStorm: { count: 50 },
  sustained: { ddBlockSize: 65536, ddCount: 512 }, // ~32 MB
  concurrent: { count: 40 },
  snapshot: { repeats: 30, primingBytes: 4096 },
  reconnect: { primingBytes: 16384 },
};

function spawnFrame(id: string, command: string, args: string[]) {
  return {
    type: "spawn",
    sessionId: id,
    command,
    args,
    token: "bench",
    cols: 120,
    rows: 40,
    cwd: "/tmp",
  };
}

// 1. Spawn storm — rapid session creation; per-op spawn→ack latency.
export async function spawnStorm(client: BenchClient) {
  const samples: number[] = [];
  for (let i = 0; i < PARAMS.spawnStorm.count; i++) {
    const id = `storm-${i}`;
    const t0 = now();
    client.send(spawnFrame(id, "/bin/sh", ["-c", "exit 0"]));
    await client.await("spawn-ack", (m) => m.sessionId === id);
    samples.push(now() - t0);
    await client.await("exit", (m) => m.sessionId === id).catch(() => {});
  }
  return { spawnAckLatencyMs: summarize(samples), count: PARAMS.spawnStorm.count };
}

// 2. Sustained high-throughput output — byte-copy throughput at the socket.
export async function sustainedThroughput(client: BenchClient) {
  const id = "flood";
  const { ddBlockSize, ddCount } = PARAMS.sustained;
  let bytes = 0;
  const off = client.on((f) => {
    if (f.meta?.type === "data" && f.meta.sessionId === id && f.body) {
      bytes += f.body.length;
      client.send({ type: "ack", sessionId: id, bytes: f.body.length });
    }
  });
  const t0 = now();
  client.send(spawnFrame(id, "/bin/dd", [`if=/dev/zero`, `bs=${ddBlockSize}`, `count=${ddCount}`]));
  await client.await("spawn-ack", (m) => m.sessionId === id);
  await client.await("exit", (m) => m.sessionId === id);
  const durationMs = now() - t0;
  off();
  const mbPerSec = bytes / 1e6 / (durationMs / 1000);
  return {
    bytesReceived: bytes,
    durationMs: Math.round(durationMs),
    throughputMBps: Math.round(mbPerSec * 100) / 100,
  };
}

// 3. Many concurrent sessions held open — daemon resident memory under load.
export async function concurrentSessions(client: BenchClient, daemonPid: number) {
  const ids: string[] = [];
  for (let i = 0; i < PARAMS.concurrent.count; i++) {
    const id = `conc-${i}`;
    ids.push(id);
    client.send(spawnFrame(id, "/bin/cat", [])); // blocks on stdin, stays alive
    await client.await("spawn-ack", (m) => m.sessionId === id);
  }
  await Bun.sleep(300); // let RSS settle
  const rss = rssKb(daemonPid);
  for (const id of ids) client.send({ type: "kill", sessionId: id });
  return { sessions: ids.length, daemonRssKb: rss };
}

// 4. Subscribe/snapshot latency — fresh client subscribes; time to snapshot frame.
export async function subscribeSnapshotLatency(sockPath: string) {
  // Prime one session with some rendered output.
  const driver = new BenchClient(sockPath);
  await driver.connect();
  const id = "snap-src";
  driver.send(
    spawnFrame(id, "/bin/sh", [
      "-c",
      `printf 'X%.0s' $(seq 1 ${PARAMS.snapshot.primingBytes}); sleep 30`,
    ]),
  );
  driver.send({ type: "ack", sessionId: id, bytes: 0 });
  driver.on((f) => {
    if (f.meta?.type === "data" && f.body)
      driver.send({ type: "ack", sessionId: id, bytes: f.body.length });
  });
  await driver.await("spawn-ack", (m) => m.sessionId === id);
  await Bun.sleep(200); // ensure priming output buffered

  const samples: number[] = [];
  for (let i = 0; i < PARAMS.snapshot.repeats; i++) {
    const c = new BenchClient(sockPath);
    await c.connect();
    const t0 = now();
    c.send({ type: "subscribe", sessionId: id });
    await c.await("snapshot", (m) => m.sessionId === id);
    samples.push(now() - t0);
    c.close();
  }
  driver.send({ type: "kill", sessionId: id });
  driver.close();
  return { snapshotLatencyMs: summarize(samples), repeats: PARAMS.snapshot.repeats };
}

// 5. Reconnect replay — non-snapshot client receives the ring-buffer replay.
export async function reconnectReplay(sockPath: string) {
  const driver = new BenchClient(sockPath);
  await driver.connect();
  const id = "replay-src";
  driver.send(
    spawnFrame(id, "/bin/sh", [
      "-c",
      `printf 'Y%.0s' $(seq 1 ${PARAMS.reconnect.primingBytes}); sleep 30`,
    ]),
  );
  driver.on((f) => {
    if (f.meta?.type === "data" && f.body)
      driver.send({ type: "ack", sessionId: id, bytes: f.body.length });
  });
  await driver.await("spawn-ack", (m) => m.sessionId === id);
  await Bun.sleep(200);

  // Reconnect WITHOUT the snapshot capability to force ring-buffer replay.
  const c = new BenchClient(sockPath, []); // no capabilities
  await c.connect();
  const t0 = now();
  c.send({ type: "subscribe", sessionId: id });
  const f = await c.await("data", (m) => m.sessionId === id);
  const latencyMs = now() - t0;
  const replayBytes = f.body?.length ?? 0;
  c.close();
  driver.send({ type: "kill", sessionId: id });
  driver.close();
  return { replayBytes, latencyMs: Math.round(latencyMs * 1000) / 1000 };
}
