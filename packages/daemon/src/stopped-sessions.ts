import * as fs from "node:fs";
import * as path from "node:path";

const MAX_ENTRIES = 2000;

export class StoppedSessionsStore {
  private cache = new Set<string>();

  constructor(private readonly filePath: string) {}

  load(): void {
    try {
      const content = fs.readFileSync(this.filePath, "utf8");
      const ids = content.split("\n").filter((l) => l.trim().length > 0);
      this.cache = new Set(ids);
    } catch {
      this.cache = new Set();
    }
  }

  add(sessionId: string): void {
    this.cache.add(sessionId);
    if (this.cache.size > MAX_ENTRIES) {
      const oldest = this.cache.values().next().value as string;
      this.cache.delete(oldest);
    }
    this.persist();
  }

  has(sessionId: string): boolean {
    return this.cache.has(sessionId);
  }

  private persist(): void {
    try {
      fs.mkdirSync(path.dirname(this.filePath), { recursive: true });
      const tmp = this.filePath + ".tmp";
      fs.writeFileSync(tmp, [...this.cache].join("\n") + "\n", "utf8");
      fs.renameSync(tmp, this.filePath);
    } catch {
      // non-fatal — in-memory cache still enforces stopped state for this run
    }
  }
}
