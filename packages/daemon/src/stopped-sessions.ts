import * as fs from "node:fs";
import * as path from "node:path";

export class StoppedSessionsStore {
  // Authoritative, never-evicted set of intentionally-stopped session ids.
  // A stopped session must stay stopped for the daemon's lifetime and across
  // restarts; ids are small so the full set is cheap to hold in memory.
  private set = new Set<string>();

  constructor(private readonly filePath: string) {}

  load(): void {
    try {
      const content = fs.readFileSync(this.filePath, "utf8");
      const ids = content.split("\n").filter((l) => l.trim().length > 0);
      this.set = new Set(ids);
    } catch {
      this.set = new Set();
    }
  }

  add(sessionId: string): void {
    if (this.set.has(sessionId)) return;
    this.set.add(sessionId);
    this.persist();
  }

  has(sessionId: string): boolean {
    return this.set.has(sessionId);
  }

  private persist(): void {
    try {
      fs.mkdirSync(path.dirname(this.filePath), { recursive: true });
      const tmp = this.filePath + ".tmp";
      const fd = fs.openSync(tmp, "w");
      try {
        fs.writeSync(fd, [...this.set].join("\n") + "\n", null, "utf8");
        fs.fsyncSync(fd);
      } finally {
        fs.closeSync(fd);
      }
      fs.renameSync(tmp, this.filePath);
    } catch {
      // non-fatal — in-memory set still enforces stopped state for this run
    }
  }
}
