// Measurement helpers: latency percentiles and daemon resident-memory sampling.

export function percentile(sortedAsc: number[], p: number): number {
  if (sortedAsc.length === 0) return 0;
  const idx = Math.min(sortedAsc.length - 1, Math.floor((p / 100) * sortedAsc.length));
  return sortedAsc[idx]!;
}

export function summarize(samplesMs: number[]): {
  p50: number;
  p95: number;
  p99: number;
  n: number;
} {
  const sorted = [...samplesMs].sort((a, b) => a - b);
  return {
    p50: round(percentile(sorted, 50)),
    p95: round(percentile(sorted, 95)),
    p99: round(percentile(sorted, 99)),
    n: sorted.length,
  };
}

export function summarizeTail(samplesMs: number[]): {
  p50: number;
  p95: number;
  p99: number;
  p999: number;
  max: number;
  n: number;
} {
  const sorted = [...samplesMs].sort((a, b) => a - b);
  return {
    p50: round(percentile(sorted, 50)),
    p95: round(percentile(sorted, 95)),
    p99: round(percentile(sorted, 99)),
    p999: round(percentile(sorted, 99.9)),
    max: round(sorted[sorted.length - 1] ?? 0),
    n: sorted.length,
  };
}

/** Least-squares slope + intercept of y over x. */
export function linFit(xs: number[], ys: number[]): { slope: number; intercept: number } {
  const n = xs.length;
  const sx = xs.reduce((a, b) => a + b, 0);
  const sy = ys.reduce((a, b) => a + b, 0);
  const sxx = xs.reduce((a, b) => a + b * b, 0);
  const sxy = xs.reduce((a, b, i) => a + b * ys[i]!, 0);
  const denom = n * sxx - sx * sx;
  if (denom === 0) return { slope: 0, intercept: ys[0] ?? 0 };
  const slope = (n * sxy - sx * sy) / denom;
  const intercept = (sy - slope * sx) / n;
  return { slope: round(slope), intercept: round(intercept) };
}

export function round(n: number): number {
  return Math.round(n * 1000) / 1000;
}

/** Resident set size of a pid in KB, or null if unavailable. */
export function rssKb(pid: number): number | null {
  try {
    const out = Bun.spawnSync(["ps", "-o", "rss=", "-p", String(pid)])
      .stdout.toString()
      .trim();
    const kb = Number(out);
    return Number.isFinite(kb) ? kb : null;
  } catch {
    return null;
  }
}

/** Cumulative CPU time (user+sys) of a pid in seconds, via `ps -o time=`. */
export function cpuTimeSec(pid: number): number | null {
  try {
    const out = Bun.spawnSync(["ps", "-o", "time=", "-p", String(pid)])
      .stdout.toString()
      .trim();
    if (!out) return null;
    // Formats: "MM:SS.cc" or "H:MM:SS" / "HH:MM:SS.cc". Sum left-to-right.
    const parts = out.split(":").map(Number);
    if (parts.some((n) => !Number.isFinite(n))) return null;
    return parts.reduce((acc, n) => acc * 60 + n, 0);
  } catch {
    return null;
  }
}

/** Median RSS over `samples` reads spaced `gapMs` apart (steady-state). */
export async function sampleRssMedian(
  pid: number,
  samples = 10,
  gapMs = 100,
): Promise<number | null> {
  const vals: number[] = [];
  for (let i = 0; i < samples; i++) {
    const v = rssKb(pid);
    if (v !== null) vals.push(v);
    await Bun.sleep(gapMs);
  }
  if (vals.length === 0) return null;
  vals.sort((a, b) => a - b);
  return vals[Math.floor(vals.length / 2)]!;
}

export function now(): number {
  return Bun.nanoseconds() / 1e6; // ms
}
