// Decisive benchmark slice — answers "is a native daemon worth it?" on the two
// metrics that map to product pain for an always-on, many-session terminal
// backend on a laptop:
//
//   A) interactive echo latency tail WHILE background sessions flood output
//      (managed-runtime GC pauses show up here as keystroke jank)
//   B) resident memory scaling vs session count (baseline + marginal KB/session)
//
// Methodology: a FRESH daemon per data point (no cross-workload pollution),
// warmup discarded, large sample counts for the tail, daemon CPU sampled, and
// the SAME explicit commands driven against both daemons (design D10). These
// workloads avoid the snapshot path, so the reference daemon participates fairly.

import { join } from "node:path";
import { existsSync } from "node:fs";
import { BenchClient } from "./client.ts";
import { launchDaemon, type LaunchedDaemon } from "./daemon.ts";
import { cpuTimeSec, linFit, now, sampleRssMedian, summarizeTail } from "./metrics.ts";

const ROOT = join(import.meta.dir, "../..");

// Fixed, reproducible parameters.
const CFG = {
  echo: {
    bgCounts: [0, 8, 32, 64],
    warmup: 200,
    samples: 2000,
    // Background flood typed INTO the login shell (real product path): a rate-
    // limited perl emitter (~300 lines/s/session of a 60-char line). perl's
    // select() sleeps sub-second without spawning per tick, so generators stay
    // mostly idle and we measure DAEMON cost, not generator CPU contention.
    bgFloodLine: `perl -e '$|=1; while(1){ print "X"x60, "\\n"; select(undef,undef,undef,0.003); }'`,
  },
  mem: {
    counts: [0, 1, 8, 32, 64, 128],
  },
};

// Every session is the user's login shell (no command) — the real product
// path. Commands are typed into the shell over the input channel.
function loginShellMeta(id: string) {
  return {
    type: "spawn",
    sessionId: id,
    args: [] as string[],
    token: "d-" + id,
    cols: 100,
    rows: 30,
    cwd: "/tmp",
  };
}

// ── Workload A: echo latency under background load ──────────────────────────

const ECHO_TIMEOUT_MS = 800;

// Returns latency ms, or null if the echo never came back within the timeout
// (a dropped frame — itself a reliability signal).
async function echoRoundTrip(c: BenchClient, id: string, seq: number): Promise<number | null> {
  const marker = `E${seq}X`;
  const t0 = now();
  const p = new Promise<number | null>((resolve) => {
    const off = c.on((f) => {
      if (
        f.meta?.type === "data" &&
        f.meta.sessionId === id &&
        f.body &&
        Buffer.from(f.body).includes(marker)
      ) {
        off();
        resolve(now() - t0);
      }
    });
    setTimeout(() => {
      off();
      resolve(null);
    }, ECHO_TIMEOUT_MS);
  });
  // Type `echo <marker>` into the login shell and time until the marker comes
  // back (the shell's line editor echoes the keystrokes). `\r` = Enter under the
  // shell's raw-mode line discipline.
  c.send({ type: "input", sessionId: id }, new TextEncoder().encode(`echo ${marker}\r`));
  return p;
}

function awaitT<T>(p: Promise<T>, ms: number, label: string): Promise<T> {
  return Promise.race([
    p,
    new Promise<T>((_, rej) => setTimeout(() => rej(new Error(`timeout: ${label}`)), ms)),
  ]);
}

async function echoUnderLoad(label: string, bin: string, bgCount: number) {
  const daemon = await launchDaemon(bin, `${label}-echo${bgCount}`);

  // Background flood on a dedicated connection; ack its data so the streams keep
  // flowing (realistic consuming client → real daemon fan-out work).
  const bg = new BenchClient(daemon.sockPath);
  await bg.connect();
  bg.on((f) => {
    if (f.meta?.type === "data" && f.body)
      bg.send({ type: "ack", sessionId: f.meta.sessionId, bytes: f.body.length });
  });
  for (let i = 0; i < bgCount; i++) {
    const id = `bg-${i}`;
    bg.send(loginShellMeta(id));
    await awaitT(
      bg.await("spawn-ack", (m) => m.sessionId === id),
      5000,
      `bg spawn ${id}`,
    );
    // Drive the flood command into the login shell.
    bg.send(
      { type: "input", sessionId: id },
      new TextEncoder().encode(CFG.echo.bgFloodLine + "\r"),
    );
  }
  if (bgCount > 0) await Bun.sleep(1500); // let shells init + flood ramp to steady rate

  // Foreground interactive login shell on its own connection.
  const fg = new BenchClient(daemon.sockPath);
  await fg.connect();
  const fgId = "fg";
  fg.on((f) => {
    if (f.meta?.type === "data" && f.meta.sessionId === fgId && f.body) {
      fg.send({ type: "ack", sessionId: fgId, bytes: f.body.length });
    }
  });
  fg.send(loginShellMeta(fgId));
  await awaitT(
    fg.await("spawn-ack", (m) => m.sessionId === fgId),
    5000,
    "fg spawn",
  );
  await Bun.sleep(800); // let the login shell finish init and reach its prompt

  // Fast pre-probe (25 echoes): detect an unreliable-delivery daemon in seconds
  // instead of grinding through hundreds of 800ms timeouts.
  let probeDrops = 0;
  for (let i = 0; i < 25; i++) {
    if ((await echoRoundTrip(fg, fgId, 900000 + i)) === null) probeDrops++;
  }
  if (probeDrops / 25 > 0.3) {
    bg.close();
    fg.close();
    daemon.stop();
    return {
      bgCount,
      p50: NaN,
      p95: NaN,
      p99: NaN,
      p999: NaN,
      max: NaN,
      n: 0,
      daemonCpuPct: NaN,
      dropPct: Math.round((probeDrops / 25) * 1000) / 10,
      unreliable: true,
    };
  }

  // Warmup, with an early reliability check.
  let warmDrops = 0;
  for (let i = 0; i < CFG.echo.warmup; i++) {
    if ((await echoRoundTrip(fg, fgId, i)) === null) warmDrops++;
  }
  const warmDropRate = warmDrops / CFG.echo.warmup;

  const samples: number[] = [];
  let drops = 0;
  const cpu0 = cpuTimeSec(daemon.pid) ?? 0;
  const wall0 = now();
  if (warmDropRate <= 0.3) {
    for (let i = 0; i < CFG.echo.samples; i++) {
      const r = await echoRoundTrip(fg, fgId, CFG.echo.warmup + i);
      if (r === null) drops++;
      else samples.push(r);
      // Abort early if delivery is badly unreliable.
      if (i >= 200 && drops / (i + 1) > 0.3) break;
    }
  }
  const wallMs = now() - wall0;
  const cpu1 = cpuTimeSec(daemon.pid) ?? 0;
  const cpuPct = wallMs > 0 ? Math.round(((cpu1 - cpu0) / (wallMs / 1000)) * 1000) / 10 : 0;

  bg.close();
  fg.close();
  daemon.stop();

  const attempted = samples.length + drops;
  const dropRate =
    attempted > 0
      ? Math.round((drops / attempted) * 1000) / 10
      : Math.round(warmDropRate * 1000) / 10;
  const unreliable = warmDropRate > 0.3 || dropRate > 5;
  const t =
    samples.length > 0
      ? summarizeTail(samples)
      : { p50: NaN, p95: NaN, p99: NaN, p999: NaN, max: NaN, n: 0 };
  return { bgCount, ...t, daemonCpuPct: cpuPct, dropPct: dropRate, unreliable };
}

// ── Workload B: memory scaling ──────────────────────────────────────────────

async function memoryScaling(label: string, bin: string, count: number) {
  const daemon: LaunchedDaemon = await launchDaemon(bin, `${label}-mem${count}`);
  const c = new BenchClient(daemon.sockPath);
  await c.connect();
  for (let i = 0; i < count; i++) {
    const id = `sh-${i}`;
    c.send(loginShellMeta(id)); // idle login shell at its prompt
    await awaitT(
      c.await("spawn-ack", (m) => m.sessionId === id),
      5000,
      `mem spawn ${id}`,
    );
  }
  await Bun.sleep(500);
  const rss = await sampleRssMedian(daemon.pid, 10, 100);
  c.close();
  daemon.stop();
  return { count, rssKb: rss };
}

// ── Orchestration ───────────────────────────────────────────────────────────

function resolveBinaries(): Record<string, string> {
  const args = process.argv.slice(2).filter((a) => a.includes("="));
  if (args.length > 0) {
    const map: Record<string, string> = {};
    for (const a of args) {
      const [l, p] = a.split("=", 2);
      if (l && p) map[l] = p;
    }
    return map;
  }
  const map: Record<string, string> = {};
  const rust = join(ROOT, "packages/daemon-pty/target/release/athing-daemon");
  if (existsSync(rust)) map["rust"] = rust;
  // The Node daemon is the working TS incumbent (the Bun daemon can't accept
  // input under Bun). Compare rust vs node by default.
  const node = process.env.ATHING_DAEMON_BIN ?? join(ROOT, "bin/athing-daemon");
  if (existsSync(node)) map["node"] = node;
  return map;
}

function pad(s: string, n: number) {
  return s.padEnd(n);
}

function fmt(v: any): string {
  if (v === undefined || v === null || (typeof v === "number" && Number.isNaN(v))) return "-";
  return String(v);
}

function renderEcho(results: Record<string, any[]>): string {
  const labels = Object.keys(results);
  const out: string[] = [];
  out.push("\nA) Interactive echo latency under background flood (ms; lower better)");
  out.push("   `echo` into a foreground login shell while N background login shells flood\n");
  for (const metric of ["p50", "p99", "p999", "max", "dropPct", "daemonCpuPct"]) {
    const titles: Record<string, string> = {
      dropPct: "echo DROP % (lost)",
      daemonCpuPct: "daemon CPU % (during)",
    };
    out.push(`  ${titles[metric] ?? "echo " + metric}`);
    out.push("  " + pad("bg sessions", 16) + labels.map((l) => pad(l, 14)).join(""));
    for (const bg of CFG.echo.bgCounts) {
      const cell = (l: string) => {
        const r = results[l]?.find((x) => x.bgCount === bg);
        return r ? fmt(r[metric]) : "-";
      };
      out.push("  " + pad(String(bg), 16) + labels.map((l) => pad(cell(l), 14)).join(""));
    }
    out.push("");
  }
  return out.join("\n");
}

function renderMem(results: Record<string, any[]>): string {
  const labels = Object.keys(results);
  const out: string[] = [];
  out.push("B) Resident memory vs idle-session count (KB; lower better)\n");
  out.push("  " + pad("idle sessions", 16) + labels.map((l) => pad(l, 14)).join(""));
  for (const n of CFG.mem.counts) {
    const cell = (l: string) => {
      const r = results[l]?.find((x) => x.count === n);
      return r ? fmt(r.rssKb) : "-";
    };
    out.push("  " + pad(String(n), 16) + labels.map((l) => pad(cell(l), 14)).join(""));
  }
  out.push("");
  const baseline = (l: string) => fmt(results[l]?.find((x) => x.count === 0)?.rssKb);
  out.push("  " + pad("baseline (0 sess)", 16) + labels.map((l) => pad(baseline(l), 14)).join(""));
  out.push(
    "  " +
      pad("marginal KB/sess", 16) +
      labels
        .map((l) => {
          const rows = (results[l] ?? []).filter((r) => typeof r.rssKb === "number");
          if (rows.length < 2) return pad("-", 14);
          const fit = linFit(
            rows.map((r) => r.count),
            rows.map((r) => r.rssKb),
          );
          return pad(String(fit.slope), 14);
        })
        .join(""),
  );
  out.push("");
  return out.join("\n");
}

async function main() {
  const binaries = resolveBinaries();
  const labels = Object.keys(binaries);
  if (labels.length === 0) {
    console.error(
      "No daemon binaries found. Build rust (cd packages/daemon-pty && cargo build --release) or pass label=path.",
    );
    process.exit(1);
  }
  if (!existsSync("/usr/bin/perl")) {
    console.error(
      "background flood needs perl (/usr/bin/perl); install perl or adjust CFG.echo.bgFloodLine",
    );
    process.exit(1);
  }
  console.error("Decisive benchmark:", labels.map((l) => `${l}=${binaries[l]}`).join("  "));

  const echo: Record<string, any[]> = {};
  const mem: Record<string, any[]> = {};
  const jsonPath = join(import.meta.dir, "decisive-run.json");
  const save = () => Bun.write(jsonPath, JSON.stringify({ binaries, echo, mem }, null, 2));

  for (const label of labels) {
    // Per-daemon isolation: a hang/failure on one daemon must not lose another's
    // completed results.
    echo[label] = [];
    for (const bg of CFG.echo.bgCounts) {
      console.error(`[${label}] echo under load: bg=${bg} ...`);
      try {
        const r = await awaitT(
          echoUnderLoad(label, binaries[label]!, bg),
          180_000,
          `${label} echo bg=${bg}`,
        );
        echo[label].push(r);
        if (r.unreliable) {
          console.error(`[${label}] bg=${bg} UNRELIABLE (drop ${r.dropPct}%) — skipping higher bg`);
          await save();
          break;
        }
      } catch (e) {
        console.error(`[${label}] echo bg=${bg} FAILED: ${e}`);
        echo[label].push({
          bgCount: bg,
          failed: String(e),
          unreliable: true,
          p50: NaN,
          p99: NaN,
          p999: NaN,
          max: NaN,
          dropPct: 100,
          daemonCpuPct: NaN,
        });
        await save();
        break;
      }
      await save();
    }
    mem[label] = [];
    for (const n of CFG.mem.counts) {
      console.error(`[${label}] memory scaling: n=${n} ...`);
      try {
        mem[label].push(
          await awaitT(memoryScaling(label, binaries[label]!, n), 120_000, `${label} mem n=${n}`),
        );
      } catch (e) {
        console.error(`[${label}] mem n=${n} FAILED: ${e}`);
        mem[label].push({ count: n, rssKb: null, failed: String(e) });
      }
      await save();
    }
  }

  const report = [
    "\n" + "=".repeat(70),
    "  Decisive daemon benchmark — fresh daemon per point, same commands",
    "=".repeat(70),
    renderEcho(echo),
    renderMem(mem),
    "=".repeat(70),
  ].join("\n");
  console.log(report);

  await Bun.write(
    join(import.meta.dir, "decisive-run.json"),
    JSON.stringify({ binaries, echo, mem }, null, 2),
  );
  console.error("JSON -> tests/benchmark/decisive-run.json");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
