import * as fs from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

export const ATHING_DIR = process.env["ATHING_DIR"]
  ? resolve(process.env["ATHING_DIR"])
  : join(homedir(), ".athing");
export const MANIFEST_PATH = join(ATHING_DIR, "daemon.json");
export const DAEMON_SOCK = join(ATHING_DIR, "daemon.sock");
export const HOOKS_SOCK = join(ATHING_DIR, "hooks.sock");

export interface ManifestData {
  pid: number;
  version: string;
}

export class Manifest {
  write(version: string): void {
    this.writeForPid(process.pid, version);
  }

  writeForPid(pid: number, version: string): void {
    fs.mkdirSync(ATHING_DIR, { recursive: true });
    const tmp = MANIFEST_PATH + ".tmp";
    fs.writeFileSync(tmp, JSON.stringify({ pid, version }), "utf8");
    fs.renameSync(tmp, MANIFEST_PATH);
  }

  remove(): void {
    try {
      fs.rmSync(MANIFEST_PATH);
    } catch {
      // already gone
    }
  }

  static read(): ManifestData | null {
    try {
      const raw = fs.readFileSync(MANIFEST_PATH, "utf8");
      return JSON.parse(raw) as ManifestData;
    } catch {
      return null;
    }
  }
}
