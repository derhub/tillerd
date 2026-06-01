import * as fs from "node:fs";

export interface SnapshotRecord {
  sessionId: string;
  pid: number;
  cwd: string;
  cols: number;
  rows: number;
  fdIndex: number;
  replayBuffer: string; // base64
}

export function writeSnapshot(snapshotPath: string, records: SnapshotRecord[]): void {
  const lines = records.map((r) => JSON.stringify(r)).join("\n") + "\n";
  const tmp = snapshotPath + ".tmp";
  fs.writeFileSync(tmp, lines, "utf8");
  fs.renameSync(tmp, snapshotPath);
}

export function readSnapshot(snapshotPath: string): SnapshotRecord[] {
  const content = fs.readFileSync(snapshotPath, "utf8");
  return content
    .split("\n")
    .filter((line) => line.trim().length > 0)
    .map((line) => JSON.parse(line) as SnapshotRecord);
}
