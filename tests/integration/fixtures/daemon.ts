//! e2e daemon fixture: spawn a real daemon against an isolated temp runtime dir,
//! wait until its socket answers, and tear it down. Self-provisioning — no
//! pre-running service and no manually exported env required.

import * as path from "node:path";
import * as os from "node:os";
import * as fs from "node:fs";

/** The release binary the `e2e` turbo task builds via `@athing/daemon-pty#build`. */
export const DAEMON_BIN = path.resolve(import.meta.dir, "../../../target/release/athing-daemon");

export interface DaemonHandle {
  /** Isolated runtime directory for this daemon (removed on stop). */
  athingDir: string;
  /** The daemon control socket inside `athingDir`. */
  sockPath: string;
  /** The resolved daemon binary path. */
  bin: string;
  /** Kill the daemon and remove the runtime directory. */
  stop(): Promise<void>;
}

async function canConnect(sockPath: string): Promise<boolean> {
  try {
    const sock = await Bun.connect({ unix: sockPath, socket: { data() {} } });
    sock.end();
    return true;
  } catch {
    return false;
  }
}

async function waitForSocket(sockPath: string, timeoutMs = 8_000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await canConnect(sockPath)) return;
    await Bun.sleep(50);
  }
  throw new Error(`daemon socket did not come up at ${sockPath} within ${timeoutMs}ms`);
}

/**
 * Spawn the daemon against a fresh temp `ATHING_DIR` and wait for its socket.
 * Throws a clear error if the binary is absent (run via `bun run e2e`, which
 * builds it through the turbo `@athing/daemon-pty#build` dependency).
 */
export async function startDaemon(): Promise<DaemonHandle> {
  if (!fs.existsSync(DAEMON_BIN)) {
    throw new Error(
      `daemon binary missing at ${DAEMON_BIN}. Run \`bun run e2e\` (builds it via the ` +
        `daemon-pty build dependency) or \`cargo build --release\` first.`,
    );
  }
  const athingDir = fs.mkdtempSync(path.join(os.tmpdir(), "athing-e2e-"));
  const proc = Bun.spawn([DAEMON_BIN], {
    env: { ...process.env, ATHING_DIR: athingDir },
    stdout: "ignore",
    stderr: "ignore",
  });
  const sockPath = path.join(athingDir, "daemon.sock");
  try {
    await waitForSocket(sockPath);
  } catch (err) {
    proc.kill();
    throw err;
  }
  return {
    athingDir,
    sockPath,
    bin: DAEMON_BIN,
    async stop() {
      proc.kill();
      await proc.exited;
      try {
        fs.rmSync(athingDir, { recursive: true, force: true });
      } catch {
        /* best effort */
      }
    },
  };
}
