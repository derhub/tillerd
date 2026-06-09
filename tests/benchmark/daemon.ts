// Launch a daemon binary in isolation (own TILLERD_DIR + socket) and wait until
// it is accepting connections. Works for any conforming binary — reference or
// Rust — selected purely by the path passed in.

import { existsSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

export interface LaunchedDaemon {
  pid: number;
  sockPath: string;
  dir: string;
  stop(): void;
}

export async function launchDaemon(binPath: string, label: string): Promise<LaunchedDaemon> {
  const dir = mkdtempSync(join(tmpdir(), `tillerd-bench-${label}-`));
  const sockPath = join(dir, "daemon.sock");

  const proc = Bun.spawn([binPath], {
    env: { ...process.env, TILLERD_DIR: dir },
    stdio: ["ignore", "ignore", "ignore"],
  });

  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (existsSync(sockPath)) {
      // Give the listener a beat to begin accepting.
      await Bun.sleep(50);
      return {
        pid: proc.pid!,
        sockPath,
        dir,
        stop() {
          try {
            proc.kill("SIGTERM");
          } catch {}
          try {
            rmSync(dir, { recursive: true, force: true });
          } catch {}
        },
      };
    }
    await Bun.sleep(50);
  }
  try {
    proc.kill("SIGKILL");
  } catch {}
  throw new Error(`daemon '${label}' (${binPath}) did not create a socket within 15s`);
}
