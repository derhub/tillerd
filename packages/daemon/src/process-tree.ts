import { spawnSync } from "node:child_process";

export interface ProcInfo {
  pid: number;
  ppid: number;
}

/**
 * Parse `ps -axo pid=,ppid=` output. The `=` suffixes suppress column headers,
 * so every non-empty line is `<pid> <ppid>` with ps's leading-space padding.
 */
export function parsePsOutput(output: string): ProcInfo[] {
  const procs: ProcInfo[] = [];
  for (const line of output.split("\n")) {
    const parts = line.trim().split(/\s+/);
    if (parts.length < 2) continue;
    const pid = Number(parts[0]);
    const ppid = Number(parts[1]);
    if (Number.isInteger(pid) && Number.isInteger(ppid)) {
      procs.push({ pid, ppid });
    }
  }
  return procs;
}

/**
 * Descendants of `root` (root excluded) via parent-chain BFS. Walking the ppid
 * edge — not the process group — reaps children that called setsid() or
 * otherwise detached into their own group and would survive a `kill(-pgid)`:
 * setsid changes the session/group but never the parent, so the ppid link
 * persists. The seen-set guards against pid reuse forming a cycle.
 */
export function collectDescendantPids(root: number, procs: readonly ProcInfo[]): number[] {
  const childrenByParent = new Map<number, number[]>();
  for (const { pid, ppid } of procs) {
    const siblings = childrenByParent.get(ppid);
    if (siblings) siblings.push(pid);
    else childrenByParent.set(ppid, [pid]);
  }
  const descendants: number[] = [];
  const seen = new Set<number>([root]);
  const queue = [root];
  while (queue.length > 0) {
    const current = queue.shift()!;
    for (const child of childrenByParent.get(current) ?? []) {
      if (seen.has(child)) continue;
      seen.add(child);
      descendants.push(child);
      queue.push(child);
    }
  }
  return descendants;
}

/**
 * Snapshot the descendant pids of `root` from the live process table. Must run
 * BEFORE `root` is killed: once root dies its descendants reparent to init and
 * the parent chain that identifies them is gone. No-op on win32.
 */
export function captureDescendants(root: number): number[] {
  if (process.platform === "win32" || root <= 0) return [];
  const result = spawnSync("ps", ["-axo", "pid=,ppid="], {
    encoding: "utf8",
    timeout: 5000,
  });
  if (result.status !== 0 || !result.stdout) return [];
  return collectDescendantPids(root, parsePsOutput(result.stdout));
}

/** Best-effort signal to each pid; pids that are already dead are ignored. */
export function killPids(pids: readonly number[], signal: NodeJS.Signals): void {
  for (const pid of pids) {
    if (pid <= 0) continue;
    try {
      process.kill(pid, signal);
    } catch {
      /* already dead */
    }
  }
}
