import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { spawn } from "node:child_process";
import { spawnSync } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";
import { AtError } from "@athing/sdk";
import { ATHING_DIR, DAEMON_SOCK, HOOKS_SOCK, Manifest } from "./manifest";
import { FrameDecoder, encodeFrame } from "./protocol/codec";
import { parseClientFrame, SUPPORTED_VERSIONS, type ClientFrame } from "./protocol/messages";
import { PtySession } from "./pty-session";
import { HookIngress } from "./hook-ingress";
import { createLogger } from "./logger";
import { DAEMON_VERSION } from "./version";
import { writeSnapshot, type SnapshotRecord } from "./snapshot";

import type { Socket } from "bun";
type BunSocket = Socket<unknown>;

interface Connection {
  socket: BunSocket;
  decoder: FrameDecoder;
  negotiated: boolean;
}

export class DaemonServer {
  private sessions = new Map<string, PtySession>();
  private connections = new Map<BunSocket, Connection>();
  private server: ReturnType<typeof Bun.listen> | null = null;
  private hookIngress: HookIngress | null = null;
  private logger = createLogger();

  constructor(
    private readonly sockPath: string = DAEMON_SOCK,
    private readonly hooksSockPath: string = HOOKS_SOCK,
  ) {}

  /** Pre-populate sessions from a handoff snapshot (successor daemon only). */
  async adoptSessions(sessions: PtySession[]): Promise<void> {
    for (const s of sessions) {
      this.sessions.set(s.sessionId, s);
      s.onExit((event) => {
        this.sessions.delete(s.sessionId);
        s.emitToSubscribers((key) => {
          this.send(key as BunSocket, {
            type: "exit",
            sessionId: s.sessionId,
            code: event.code,
            signal: event.signal,
          });
        });
      });
    }
  }

  async start(): Promise<void> {
    for (const sock of [this.sockPath, this.hooksSockPath]) {
      if (fs.existsSync(sock)) {
        try {
          fs.rmSync(sock);
        } catch {
          /* ignore */
        }
      }
    }

    const logger = this.logger;
    const sessions = this.sessions;
    const connections = this.connections;

    this.hookIngress = new HookIngress({
      socketPath: this.hooksSockPath,
      getToken: (sessionId) => sessions.get(sessionId)?.token ?? null,
      onHook: (sessionId, payload) => this.relayHook(sessionId, payload),
      logger,
    });
    this.hookIngress.start();

    this.server = Bun.listen<Connection>({
      unix: this.sockPath,
      socket: {
        open: (socket) => {
          connections.set(socket, { socket, decoder: new FrameDecoder(), negotiated: false });
        },
        data: (socket, chunk) => {
          const conn = connections.get(socket);
          if (!conn) return;
          const raw =
            typeof chunk === "string"
              ? Buffer.from(chunk, "utf8")
              : Buffer.from(chunk as unknown as ArrayBuffer);
          for (const { meta, body } of conn.decoder.push(raw)) {
            this.handleFrame(conn, meta, body).catch((err) =>
              logger.warn("frame handler error", { err: String(err) }),
            );
          }
        },
        close: (socket) => {
          const conn = connections.get(socket);
          if (conn) {
            for (const session of sessions.values()) session.removeSubscriber(socket);
            connections.delete(socket);
          }
        },
        error: (_socket, err) => {
          logger.warn("socket error", { err: String(err) });
        },
      },
    });

    logger.info("daemon started", { sock: this.sockPath });
  }

  private send(socket: BunSocket, meta: unknown, body?: Buffer): void {
    socket.write(encodeFrame(meta, body));
  }

  private sendError(socket: BunSocket, code: string, message: string, sessionId?: string): void {
    this.send(socket, { type: "error", code, message, ...(sessionId ? { sessionId } : {}) });
  }

  private async handleFrame(conn: Connection, meta: unknown, body: Buffer | null): Promise<void> {
    const { socket } = conn;

    if (!conn.negotiated) {
      const hello = meta as { type?: string; versions?: unknown };
      if (hello?.type !== "hello") {
        this.sendError(socket, "EPROTO", "expected hello");
        socket.end();
        return;
      }
      const versions = Array.isArray(hello.versions) ? (hello.versions as number[]) : [];
      const chosen = SUPPORTED_VERSIONS.find((v) => versions.includes(v)) ?? null;
      if (chosen === null) {
        this.sendError(
          socket,
          "EVERSION",
          `no compatible version; supported: ${SUPPORTED_VERSIONS.join(",")}`,
        );
        socket.end();
        return;
      }
      conn.negotiated = true;
      this.send(socket, { type: "hello-ack", version: chosen, daemonVersion: DAEMON_VERSION });
      return;
    }

    const msg = parseClientFrame(meta);
    if (!msg) {
      this.sendError(socket, "EPARSE", "malformed frame");
      return;
    }

    await this.dispatch(conn, msg, body);
  }

  private async dispatch(conn: Connection, msg: ClientFrame, body: Buffer | null): Promise<void> {
    const { socket } = conn;

    switch (msg.type) {
      case "list": {
        this.send(socket, { type: "list-ack", ids: [...this.sessions.keys()] });
        break;
      }

      case "spawn": {
        if (this.sessions.has(msg.sessionId)) {
          this.sendError(socket, "EEXIST", "session already exists", msg.sessionId);
          return;
        }
        const session = new PtySession({
          sessionId: msg.sessionId,
          token: msg.token,
          command: msg.command,
          args: msg.args,
          flags: msg.flags,
          hookSocketPath: msg.hookSocketPath,
          cols: msg.cols,
          rows: msg.rows,
          cwd: msg.cwd,
        });
        this.sessions.set(msg.sessionId, session);
        session.onExit((event) => {
          this.sessions.delete(msg.sessionId);
          session.emitToSubscribers((key) => {
            const s = key as Socket<unknown>;
            this.send(s, {
              type: "exit",
              sessionId: msg.sessionId,
              code: event.code,
              signal: event.signal,
            });
          });
        });
        session.addSubscriber(
          socket,
          (bytes) => this.sendData(socket, msg.sessionId, bytes),
          65_536,
        );
        let pid: number;
        try {
          pid = session.start();
        } catch (err) {
          this.sessions.delete(msg.sessionId);
          const atErr = err instanceof AtError ? err : new AtError("SpawnFailed", String(err));
          this.sendError(socket, atErr.kind, atErr.message, msg.sessionId);
          break;
        }
        this.logger.info("spawn-ack", { sessionId: msg.sessionId, pid });
        this.send(socket, { type: "spawn-ack", sessionId: msg.sessionId, pid });
        break;
      }

      case "kill": {
        const s = this.sessions.get(msg.sessionId);
        if (s) await s.kill();
        break;
      }

      case "input": {
        if (body) this.sessions.get(msg.sessionId)?.write(body);
        break;
      }

      case "interrupt": {
        this.sessions.get(msg.sessionId)?.interrupt();
        break;
      }

      case "resize": {
        this.sessions.get(msg.sessionId)?.resize(msg.cols, msg.rows);
        break;
      }

      case "subscribe": {
        const s = this.sessions.get(msg.sessionId);
        if (!s) {
          this.logger.warn("subscribe: session not found", { sessionId: msg.sessionId });
          this.sendError(socket, "ENOTFOUND", "unknown session", msg.sessionId);
          return;
        }
        const replay = s.getReplayBytes();
        const creditBoost = Math.max(65536, replay.length + 65536);
        this.logger.info("subscribe ok", {
          sessionId: msg.sessionId,
          replayBytes: replay.length,
          credit: creditBoost,
        });
        s.addSubscriber(
          socket,
          (bytes) => {
            this.logger.info("data →client", { sessionId: msg.sessionId, bytes: bytes.length });
            this.sendData(socket, msg.sessionId, bytes);
          },
          creditBoost,
        );
        if (replay.length > 0) {
          this.sendData(socket, msg.sessionId, replay);
        }
        break;
      }

      case "unsubscribe": {
        this.sessions.get(msg.sessionId)?.removeSubscriber(socket);
        break;
      }

      case "ack": {
        this.sessions.get(msg.sessionId)?.addCredit(socket, msg.bytes);
        break;
      }

      case "upgrade": {
        this.prepareUpgrade().catch((err) =>
          this.logger.warn("upgrade failed", { err: String(err) }),
        );
        break;
      }
    }
  }

  private sendData(socket: BunSocket, sessionId: string, bytes: Uint8Array): void {
    this.send(socket, { type: "data", sessionId, bodyLen: bytes.length }, Buffer.from(bytes));
  }

  private relayHook(sessionId: string, payload: unknown): void {
    const session = this.sessions.get(sessionId);
    if (!session) return;
    session.emitToSubscribers((key) =>
      this.send(key as BunSocket, { type: "hook", sessionId, payload }),
    );
  }

  async prepareUpgrade(): Promise<void> {
    const sessions = [...this.sessions.values()];

    const records: SnapshotRecord[] = [];
    const masterFds: number[] = [];

    for (let i = 0; i < sessions.length; i++) {
      const s = sessions[i]!;
      const fdIndex = 4 + i;
      let masterFd: number;
      try {
        masterFd = s.getMasterFd();
      } catch (err) {
        this.logger.warn("upgrade: cannot get master fd, skipping session", {
          sessionId: s.sessionId,
          err: String(err),
        });
        continue;
      }
      masterFds.push(masterFd);
      records.push({
        sessionId: s.sessionId,
        pid: s.pid,
        cwd: s.cwd,
        cols: s.cols,
        rows: s.rows,
        fdIndex,
        replayBuffer: Buffer.from(s.getReplayBytes()).toString("base64"),
      });
    }

    const snapshotPath = path.join(ATHING_DIR, `snapshot-upgrade.ndjson`);
    try {
      writeSnapshot(snapshotPath, records);
    } catch (err) {
      this.logger.warn("upgrade: snapshot write failed", { err: String(err) });
      return;
    }

    let daemonBin: string;
    try {
      daemonBin = resolveDaemonBinary();
    } catch (err) {
      this.logger.warn("upgrade: cannot resolve daemon binary", { err: String(err) });
      return;
    }
    // stdio: [ignore, inherit, inherit, ipc, ...masterFds]
    const stdioLayout: Array<"ignore" | "inherit" | "ipc" | number> = [
      "ignore",
      "inherit",
      "inherit",
      "ipc",
      ...masterFds,
    ];

    const successor = spawn(
      daemonBin,
      ["--handoff", `--snapshot=${snapshotPath}`, `--socket=${this.sockPath}`],
      { detached: true, stdio: stdioLayout as Parameters<typeof spawn>[2]["stdio"] },
    );

    // Bun's bun-types doesn't expose EventEmitter.on() on ChildProcess but the runtime has it.
    const ee = successor as unknown as {
      on(event: string, cb: (...args: unknown[]) => void): void;
    };
    const ackResult = await new Promise<"ack" | "nak" | "timeout">((resolve) => {
      const timer = setTimeout(() => resolve("timeout"), 10_000);
      ee.on("message", (msg: unknown) => {
        const m = msg as { type?: string };
        if (m?.type === "upgrade-ack") {
          clearTimeout(timer);
          resolve("ack");
        } else if (m?.type === "upgrade-nak") {
          clearTimeout(timer);
          resolve("nak");
        }
      });
      ee.on("error", () => {
        clearTimeout(timer);
        resolve("nak");
      });
    });

    if (ackResult !== "ack") {
      this.logger.warn("upgrade aborted", { reason: ackResult });
      try {
        successor.kill("SIGKILL");
      } catch {
        /* already dead */
      }
      return;
    }

    // Successor acknowledged — hand off the manifest and exit.
    const successorPid = successor.pid!;
    new Manifest().writeForPid(successorPid, DAEMON_VERSION);
    this.server?.stop();
    process.exit(0);
  }

  async shutdown(): Promise<void> {
    const kills = [...this.sessions.values()].map((s) => s.kill());
    await Promise.allSettled(kills);
    this.sessions.clear();
    this.hookIngress?.stop();
    this.server?.stop();
    for (const sock of [this.sockPath, this.hooksSockPath]) {
      try {
        fs.rmSync(sock);
      } catch {
        /* ignore */
      }
    }
  }
}

function resolveDaemonBinary(): string {
  const envBin = process.env["ATHING_DAEMON_BIN"];
  if (envBin) {
    const abs = path.resolve(envBin);
    if (fs.existsSync(abs)) return abs;
  }
  const localBin = join(process.cwd(), "bin", "athing-daemon");
  if (fs.existsSync(localBin)) return localBin;
  const shell = process.env["SHELL"] ?? "/bin/sh";
  const result = spawnSync(shell, ["-lc", "which athing-daemon"], {
    encoding: "utf8",
    timeout: 5000,
  });
  if (result.status === 0 && result.stdout.trim()) return result.stdout.trim();
  const userBin = join(homedir(), ".local", "bin", "athing-daemon");
  if (fs.existsSync(userBin)) return userBin;
  throw new Error(
    "Cannot resolve athing-daemon binary. Run `bun run build` in packages/daemon or set ATHING_DAEMON_BIN.",
  );
}
