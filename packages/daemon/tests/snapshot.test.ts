import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { writeSnapshot, readSnapshot, type SnapshotRecord } from "../src/snapshot";

function tmpFile(): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "athing-snapshot-"));
  return path.join(dir, "snapshot.ndjson");
}

const SAMPLE: SnapshotRecord[] = [
  {
    sessionId: "s1",
    token: "tok-s1",
    pid: 1234,
    cwd: "/home/user",
    cols: 80,
    rows: 24,
    fdIndex: 4,
    replayBuffer: "aGVsbG8=",
  },
  { sessionId: "s2", token: "tok-s2", pid: 5678, cwd: "/tmp", cols: 120, rows: 40, fdIndex: 5, replayBuffer: "" },
];

describe("snapshot serialisation", () => {
  const tmpFiles: string[] = [];

  afterEach(() => {
    for (const f of tmpFiles.splice(0)) {
      const dir = path.dirname(f);
      try {
        fs.rmSync(dir, { recursive: true, force: true });
      } catch {}
    }
  });

  test("round-trip: write then read returns equal records", () => {
    const p = tmpFile();
    tmpFiles.push(p);
    writeSnapshot(p, SAMPLE);
    const records = readSnapshot(p);
    expect(records).toHaveLength(2);
    expect(records[0]).toEqual(SAMPLE[0]);
    expect(records[1]).toEqual(SAMPLE[1]);
  });

  test("empty snapshot writes and reads back as empty array", () => {
    const p = tmpFile();
    tmpFiles.push(p);
    writeSnapshot(p, []);
    const records = readSnapshot(p);
    expect(records).toHaveLength(0);
  });

  test("atomic write leaves no partial file visible", () => {
    const p = tmpFile();
    tmpFiles.push(p);
    const tmp = p + ".tmp";
    // File should not exist before write.
    expect(fs.existsSync(p)).toBe(false);
    writeSnapshot(p, SAMPLE);
    // tmp file should be gone (renamed to final path).
    expect(fs.existsSync(tmp)).toBe(false);
    expect(fs.existsSync(p)).toBe(true);
  });

  test("replayBuffer field survives round-trip (binary-safe base64)", () => {
    const original = new Uint8Array([0x0, 0x0a, 0xff, 0x80]);
    const b64 = Buffer.from(original).toString("base64");
    const p = tmpFile();
    tmpFiles.push(p);
    writeSnapshot(p, [{ ...SAMPLE[0]!, replayBuffer: b64 }]);
    const [record] = readSnapshot(p);
    const decoded = Buffer.from(record!.replayBuffer, "base64");
    expect(Array.from(decoded)).toEqual(Array.from(original));
  });
});
