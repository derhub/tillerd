import { Manifest } from "./manifest";
import { DaemonServer } from "./server";
import { DAEMON_VERSION } from "./version";
import { readSnapshot } from "./snapshot";
import { PtySession } from "./pty-session";
import { PtyTransport } from "./pty-transport";
import { createLogger } from "@athing/logger";

async function main() {
  if (process.argv.includes("--handoff")) {
    await runHandoffReceiver();
    return;
  }

  const manifest = new Manifest();
  const server = new DaemonServer();

  // Start server first so the manifest is only written when the socket is ready.
  await server.start();
  manifest.write(DAEMON_VERSION);

  const cleanup = async () => {
    await server.shutdown();
    manifest.remove();
    process.exit(0);
  };

  // Signal handlers: call cleanup() and let process.exit(0) inside it terminate the process.
  // The event loop stays alive while server.shutdown() awaits, so cleanup completes normally.
  process.on("SIGTERM", () => {
    void cleanup();
  });
  process.on("SIGINT", () => {
    void cleanup();
  });

  process.on("uncaughtException", (err) => {
    console.error("uncaught exception", err);
    manifest.remove();
    process.exit(1);
  });
}

async function runHandoffReceiver(): Promise<void> {
  const logger = createLogger();

  const snapshotArg = process.argv.find((a) => a.startsWith("--snapshot="));
  const socketArg = process.argv.find((a) => a.startsWith("--socket="));

  if (!snapshotArg || !socketArg) {
    console.error("--handoff requires --snapshot=<path> and --socket=<path>");
    process.exit(1);
  }

  const snapshotPath = snapshotArg.slice("--snapshot=".length);
  const sockPath = socketArg.slice("--socket=".length);

  let records;
  try {
    records = readSnapshot(snapshotPath);
  } catch (err) {
    logger.warn("handoff: failed to read snapshot", { err: String(err) });
    if (process.send) process.send({ type: "upgrade-nak", reason: "snapshot read failed" });
    process.exit(1);
  }

  // Adopt each inherited PTY master fd from process.stdio.
  const adoptedSessions: PtySession[] = [];

  for (const record of records) {
    try {
      // The fd is the raw file descriptor index in this process (same as fdIndex from snapshot).
      const fd = record.fdIndex;

      const transport = PtyTransport.adoptFromFd(fd, record.pid, {
        logger: createLogger(record.sessionId),
        shutdownGraceMs: 5_000,
      });

      const session = PtySession.fromAdoptedTransport(record.sessionId, transport, {
        replayBuffer: Buffer.from(record.replayBuffer, "base64"),
        cwd: record.cwd,
        cols: record.cols,
        rows: record.rows,
        pid: record.pid,
      });

      adoptedSessions.push(session);
    } catch (err) {
      logger.warn("handoff: failed to adopt session", {
        sessionId: record.sessionId,
        err: String(err),
      });
    }
  }

  // Determine hooks socket path from the socket path (sibling file).
  const path = await import("node:path");
  const dir = path.dirname(sockPath);
  const hooksSockPath = path.join(dir, "hooks.sock");

  const server = new DaemonServer(sockPath, hooksSockPath);
  await server.adoptSessions(adoptedSessions);
  await server.start();

  const manifest = new Manifest();
  manifest.write(DAEMON_VERSION);

  // Send upgrade-ack to predecessor via IPC channel (fd 3).
  if (process.send) {
    process.send({ type: "upgrade-ack", successorPid: process.pid });
  }

  logger.info("handoff complete", { sessions: adoptedSessions.length });

  const cleanup = async () => {
    await server.shutdown();
    manifest.remove();
    process.exit(0);
  };

  process.on("SIGTERM", () => {
    void cleanup();
  });
  process.on("SIGINT", () => {
    void cleanup();
  });
  process.on("uncaughtException", (err) => {
    console.error("uncaught exception", err);
    manifest.remove();
    process.exit(1);
  });
}

main().catch((err) => {
  console.error(err);
  // server.start() may have failed before manifest was written; best-effort remove.
  try {
    new Manifest().remove();
  } catch {}
  process.exit(1);
});
