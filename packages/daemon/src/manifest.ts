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
  constructor(private readonly dir: string = ATHING_DIR) {}

  private get manifestPath(): string {
    return join(this.dir, "daemon.json");
  }

  write(version: string): void {
    this.writeForPid(process.pid, version);
  }

  writeForPid(pid: number, version: string): void {
    fs.mkdirSync(this.dir, { recursive: true });
    const tmp = this.manifestPath + ".tmp";
    fs.writeFileSync(tmp, JSON.stringify({ pid, version }), "utf8");
    fs.renameSync(tmp, this.manifestPath);
  }

  remove(): void {
    try {
      fs.rmSync(this.manifestPath);
    } catch {
      // already gone
    }
  }

  static read(dir = ATHING_DIR): ManifestData | null {
    const manifestPath = join(dir, "daemon.json");
    try {
      const raw = fs.readFileSync(manifestPath, "utf8");
      return JSON.parse(raw) as ManifestData;
    } catch {
      return null;
    }
  }
}
