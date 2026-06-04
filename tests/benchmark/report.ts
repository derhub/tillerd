// Single comparative report: each metric attributed to its workload and to the
// daemon binary under test, presented side by side.

export type BinaryResult = Record<string, any>;

export function renderReport(results: Record<string, BinaryResult>): string {
  const labels = Object.keys(results);
  const lines: string[] = [];
  const pad = (s: string, n: number) => s.padEnd(n);
  const col = 22;

  lines.push("");
  lines.push("=".repeat(20 + col * labels.length));
  lines.push("  Daemon benchmark — comparative report");
  lines.push("  driven over the real control socket; same workloads per binary");
  lines.push("=".repeat(20 + col * labels.length));

  const header = pad("  metric", 34) + labels.map((l) => pad(l, col)).join("");
  const row = (metric: string, get: (r: BinaryResult) => string) =>
    lines.push(pad("  " + metric, 34) + labels.map((l) => pad(get(results[l]!), col)).join(""));

  const section = (title: string) => {
    lines.push("");
    lines.push("  " + title);
    lines.push("  " + "-".repeat(title.length));
    lines.push(header);
  };

  section("Spawn storm (rapid session creation)");
  row("sessions", (r) => String(r.spawnStorm?.count ?? "-"));
  row("spawn→ack p50 (ms)", (r) => String(r.spawnStorm?.spawnAckLatencyMs?.p50 ?? "-"));
  row("spawn→ack p95 (ms)", (r) => String(r.spawnStorm?.spawnAckLatencyMs?.p95 ?? "-"));
  row("spawn→ack p99 (ms)", (r) => String(r.spawnStorm?.spawnAckLatencyMs?.p99 ?? "-"));

  section("Sustained high-throughput output");
  row("bytes received", (r) => String(r.sustained?.bytesReceived ?? "-"));
  row("duration (ms)", (r) => String(r.sustained?.durationMs ?? "-"));
  row("byte-copy throughput (MB/s)", (r) => String(r.sustained?.throughputMBps ?? "-"));

  section("Many concurrent sessions held open");
  row("sessions", (r) => String(r.concurrent?.sessions ?? "-"));
  row("daemon resident memory (KB)", (r) => String(r.concurrent?.daemonRssKb ?? "-"));

  section("Subscribe / snapshot latency");
  row("snapshot build p50 (ms)", (r) => String(r.snapshot?.snapshotLatencyMs?.p50 ?? "-"));
  row("snapshot build p95 (ms)", (r) => String(r.snapshot?.snapshotLatencyMs?.p95 ?? "-"));
  row("snapshot build p99 (ms)", (r) => String(r.snapshot?.snapshotLatencyMs?.p99 ?? "-"));

  section("Reconnect replay (ring buffer)");
  row("replay bytes", (r) => String(r.reconnect?.replayBytes ?? "-"));
  row("replay latency (ms)", (r) => String(r.reconnect?.latencyMs ?? "-"));

  lines.push("");
  lines.push("=".repeat(20 + col * labels.length));
  lines.push("");
  return lines.join("\n");
}
