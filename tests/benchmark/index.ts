// Comparative benchmark runner. Drives one or more daemon binaries through the
// identical workload sequence over the real control socket and emits a single
// side-by-side report.
//
// Usage:
//   bun tests/benchmark/index.ts                       # auto-detect rust + reference
//   bun tests/benchmark/index.ts rust=/path ref=/path  # explicit label=path pairs
//
// Binary selection (auto):
//   rust  -> packages/daemon-pty/target/release/tillerd-daemon
//   node  -> $TILLERD_DAEMON_BIN, else ./bin/tillerd-daemon (if present)

import { existsSync } from "node:fs";
import { join } from "node:path";
import { BenchClient } from "./client.ts";
import { launchDaemon } from "./daemon.ts";
import { renderReport, type BinaryResult } from "./report.ts";
import {
  spawnStorm,
  sustainedThroughput,
  concurrentSessions,
  subscribeSnapshotLatency,
  reconnectReplay,
} from "./workloads.ts";

const ROOT = join(import.meta.dir, "../..");

function resolveBinaries(): Record<string, string> {
  const args = process.argv.slice(2).filter((a) => a.includes("="));
  if (args.length > 0) {
    const map: Record<string, string> = {};
    for (const a of args) {
      const [label, path] = a.split("=", 2);
      if (label && path) map[label] = path;
    }
    return map;
  }
  const map: Record<string, string> = {};
  const rust = join(ROOT, "packages/daemon-pty/target/release/tillerd-daemon");
  if (existsSync(rust)) map["rust"] = rust;
  // The Node daemon is the working TS incumbent (Bun can't accept PTY input).
  const node = process.env.TILLERD_DAEMON_BIN ?? join(ROOT, "bin/tillerd-daemon");
  if (existsSync(node)) map["node"] = node;
  return map;
}

const WORKLOAD_TIMEOUT_MS = 30_000;

// Bound each workload so a stall on one daemon records a timeout instead of
// blocking the whole comparative run.
async function withTimeout<T>(
  label: string,
  name: string,
  fn: () => Promise<T>,
): Promise<T | { timedOut: true }> {
  console.error(`[${label}] ${name}...`);
  let timer: any;
  const timeout = new Promise<{ timedOut: true }>((resolve) => {
    timer = setTimeout(() => {
      console.error(`[${label}] ${name} TIMED OUT after ${WORKLOAD_TIMEOUT_MS}ms`);
      resolve({ timedOut: true });
    }, WORKLOAD_TIMEOUT_MS);
  });
  try {
    return await Promise.race([fn(), timeout]);
  } catch (err) {
    console.error(`[${label}] ${name} ERROR: ${err}`);
    return { timedOut: true };
  } finally {
    clearTimeout(timer);
  }
}

async function runBinary(label: string, binPath: string): Promise<BinaryResult> {
  const daemon = await launchDaemon(binPath, label);
  const result: BinaryResult = {};
  try {
    const driver = new BenchClient(daemon.sockPath);
    await driver.connect();

    result.spawnStorm = await withTimeout(label, "spawn storm", () => spawnStorm(driver));
    result.sustained = await withTimeout(label, "sustained throughput", () =>
      sustainedThroughput(driver),
    );
    result.concurrent = await withTimeout(label, "concurrent sessions", () =>
      concurrentSessions(driver, daemon.pid),
    );

    driver.close();

    result.snapshot = await withTimeout(label, "subscribe/snapshot latency", () =>
      subscribeSnapshotLatency(daemon.sockPath),
    );
    result.reconnect = await withTimeout(label, "reconnect replay", () =>
      reconnectReplay(daemon.sockPath),
    );
  } finally {
    daemon.stop();
  }
  return result;
}

async function main() {
  const binaries = resolveBinaries();
  const labels = Object.keys(binaries);
  if (labels.length === 0) {
    console.error(
      "No daemon binaries found. Build the Rust daemon (cd packages/daemon-pty && cargo build --release)",
    );
    console.error(
      "or pass explicit label=path pairs, or set TILLERD_DAEMON_BIN for the reference daemon.",
    );
    process.exit(1);
  }
  console.error("Benchmarking:", labels.map((l) => `${l}=${binaries[l]}`).join("  "));

  const results: Record<string, BinaryResult> = {};
  for (const label of labels) {
    results[label] = await runBinary(label, binaries[label]!);
  }

  const report = renderReport(results);
  console.log(report);

  // Also emit machine-readable JSON for archival.
  const jsonPath = join(import.meta.dir, "last-run.json");
  await Bun.write(jsonPath, JSON.stringify({ binaries, results }, null, 2));
  console.error(`JSON written to ${jsonPath}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
