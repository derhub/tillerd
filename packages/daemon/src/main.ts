import path from "node:path";
import { Manifest } from "./manifest";
import { DaemonServer } from "./server";
import { DAEMON_VERSION } from "./version";
import { readSnapshot } from "./snapshot";
import { PtySession } from "./pty-session";
import { PtyTransport } from "./pty-transport";
import { createLogger } from "@athing/logger";
import { installLoginShellEnv } from "./shell-env";

function argValue(flag: string): string | undefined {
  return process.argv.find((a) => a.startsWith(`${flag}=`))?.slice(flag.length + 1);
}

function registerHandlers(server: DaemonServer, manifest: Manifest) {
  const cleanup = async () => {
    await server.shutdown();
    manifest.remove();
    process.exit(0);
  };

  process.on("SIGTERM", () => void cleanup());
  process.on("SIGINT", () => void cleanup());
  process.on("uncaughtException", (err) => {
    console.error("uncaught exception", err);
    manifest.remove();
    process.exit(1);
  });
}

async function main() {
  if (process.argv.includes("--handoff")) {
    await runHandoffReceiver();
    return;
  }

  installLoginShellEnv();

  const manifest = new Manifest();
  const server = new DaemonServer();

  await server.start();
  manifest.write(DAEMON_VERSION);
  registerHandlers(server, manifest);
}

async function runHandoffReceiver(): Promise<void> {
  const logger = createLogger();

  const snapshotPath = argValue("--snapshot");
  const sockPath = argValue("--socket");

  if (!snapshotPath || !sockPath) {
    console.error("--handoff requires --snapshot=<path> and --socket=<path>");
    process.exit(1);
  }

  let records;
  try {
    records = readSnapshot(snapshotPath);
  } catch (err) {
    logger.warn("handoff: failed to read snapshot", { err: String(err) });
    process.send?.({ type: "upgrade-nak", reason: "snapshot read failed" });
    process.exit(1);
  }

  const adoptedSessions: PtySession[] = [];

  for (const record of records) {
    try {
      const transport = PtyTransport.adoptFromFd(record.fdIndex, record.pid, {
        logger: createLogger(record.sessionId),
        shutdownGraceMs: 5_000,
      });

      adoptedSessions.push(
        PtySession.fromAdoptedTransport(record.sessionId, transport, {
          replayBuffer: Buffer.from(record.replayBuffer, "base64"),
          cwd: record.cwd,
          cols: record.cols,
          rows: record.rows,
          pid: record.pid,
        })
      );
    } catch (err) {
      logger.warn("handoff: failed to adopt session", {
        sessionId: record.sessionId,
        err: String(err),
      });
    }
  }

  const hooksSockPath = path.join(path.dirname(sockPath), "hooks.sock");
  const server = new DaemonServer(sockPath, hooksSockPath);
  await server.adoptSessions(adoptedSessions);
  await server.start();

  const manifest = new Manifest();
  manifest.write(DAEMON_VERSION);

  process.send?.({ type: "upgrade-ack", successorPid: process.pid });
  logger.info("handoff complete", { sessions: adoptedSessions.length });

  registerHandlers(server, manifest);
}

main().catch((err) => {
  console.error(err);
  // server.start() may have failed before manifest was written; best-effort remove.
  try {
    new Manifest().remove();
  } catch {}
  process.exit(1);
});
