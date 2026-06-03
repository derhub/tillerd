import * as pty from "node-pty";
import type { IPty } from "node-pty";
import * as net from "node:net";
import type { ExitEvent } from "@athing/sdk";
import { AtError } from "@athing/sdk";
import type { Logger } from "@athing/logger";
import { resolveBinary } from "./resolve";

const INTERRUPT_KEY = "\x1b";

const DEFAULT_COLS = 220;
const DEFAULT_ROWS = 50;

export interface PtyTransportOptions {
  command: string;
  args: string[];
  cwd: string;
  env: Record<string, string>;
  cols?: number;
  rows?: number;
  logger: Logger;
  captureRawIo?: boolean;
  shutdownGraceMs: number;
}

type DataHandler = (bytes: Uint8Array) => void;
type ExitHandler = (event: ExitEvent) => void;

export class PtyTransport {
  private ptyProcess: IPty | null = null;
  private ptyResumeSignal: (() => void) | null = null;
  private adoptedSocket: net.Socket | null = null;
  private adoptedFd_: number | null = null;
  private adoptedPid_: number | null = null;
  private dataHandlers = new Set<DataHandler>();
  private exitHandlers = new Set<ExitHandler>();
  private killed = false;
  private paused_ = false;
  private killTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(private readonly opts: PtyTransportOptions) {}

  /**
   * Attach to an already-open PTY master fd inherited from a predecessor daemon.
   * Does not spawn a new process — the slave-side process is already running.
   */
  static adoptFromFd(
    fd: number,
    pid: number,
    opts: Pick<PtyTransportOptions, "logger" | "shutdownGraceMs">,
  ): PtyTransport {
    const dummy: PtyTransportOptions = {
      command: "",
      args: [],
      cwd: "/",
      env: {},
      logger: opts.logger,
      shutdownGraceMs: opts.shutdownGraceMs,
    };
    const t = new PtyTransport(dummy);
    t.adoptedFd_ = fd;
    t.adoptedPid_ = pid;

    const socket = new net.Socket({ fd, readable: true, writable: true, allowHalfOpen: true });
    t.adoptedSocket = socket;

    socket.on("data", (data: Buffer) => {
      if (t.paused_) return;
      for (const h of t.dataHandlers) h(new Uint8Array(data));
    });
    socket.on("end", () => {
      t.adoptedSocket = null;
      for (const h of t.exitHandlers) h({ code: null, signal: null });
    });
    socket.on("error", (err) => {
      opts.logger.warn("adopted pty socket error", { err: String(err) });
    });

    return t;
  }

  spawn(): number {
    if (this.adoptedFd_ !== null) return this.adoptedPid_!;
    const binary = resolveBinary(this.opts.command);
    this.opts.logger.info("spawning pty", { binary, args: this.opts.args });

    // Build a minimal safe env — passing the full process.env can cause
    // posix_spawnp to fail (E2BIG, null bytes, or Bun-proxy artefacts).
    const safeEnv: Record<string, string> = {
      PATH: process.env["PATH"] ?? "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
      HOME: process.env["HOME"] ?? "",
      USER: process.env["USER"] ?? "",
      LOGNAME: process.env["LOGNAME"] ?? process.env["USER"] ?? "",
      SHELL: process.env["SHELL"] ?? binary,
      LANG: process.env["LANG"] ?? "en_US.UTF-8",
      TERM: "xterm-256color",
      COLORTERM: "truecolor",
      ...(process.env["SSH_AUTH_SOCK"] ? { SSH_AUTH_SOCK: process.env["SSH_AUTH_SOCK"] } : {}),
      ...this.opts.env,
    };

    this.opts.logger.info("pty spawn params", {
      binary,
      args: this.opts.args,
      cwd: this.opts.cwd,
      envKeys: Object.keys(safeEnv).join(","),
    });

    const ptyOpts = {
      name: "xterm-256color",
      cols: this.opts.cols ?? DEFAULT_COLS,
      rows: this.opts.rows ?? DEFAULT_ROWS,
      cwd: this.opts.cwd,
      env: safeEnv,
    };

    // When no args are passed the caller wants an interactive terminal (raw shell).
    // Spawn the binary directly so it runs as an interactive (login) session rather
    // than nested inside a second shell via -lc, which causes posix_spawnp failures
    // on some systems and prevents proper interactive-shell initialisation.
    // When args are present (agent CLI flags), we need the login-shell wrapper so
    // the binary can be found via PATH even on non-login environments.
    const proc =
      this.opts.args.length === 0
        ? pty.spawn(
            binary,
            /\/(sh|bash|zsh|fish|dash|csh|tcsh|ksh)$/.test(binary) ? ["-l"] : [],
            ptyOpts,
          )
        : pty.spawn(
            process.env["SHELL"] ?? "/bin/sh",
            ["-lc", `exec ${binary} ${this.opts.args.join(" ")}`],
            ptyOpts,
          );

    this.ptyProcess = proc;

    // Bun's libuv watcher and net.Socket({fd}) both fail to deliver events for
    // node-pty master fds created by the native addon. Use Bun.file("/dev/fd/N")
    // in an async loop instead — Bun's native file streaming works on these fds.
    const masterFd = (proc as unknown as { _fd?: number })._fd;
    if (typeof masterFd === "number") {
      this.opts.logger.info("pty fd", { masterFd });
      this.startBunReadLoop(masterFd);
    } else {
      this.opts.logger.warn("pty _fd not available, falling back to proc.onData");
      proc.onData((data) => {
        if (this.paused_) return;
        const bytes = Buffer.from(data, "binary");
        if (this.opts.captureRawIo) {
          this.opts.logger.debug("pty.out", { bytes: bytes.toString("hex") });
        }
        for (const h of this.dataHandlers) h(bytes);
      });
    }

    proc.onExit(({ exitCode, signal }) => {
      this.opts.logger.info("pty.exit", { exitCode, signal });
      this.cleanup();
      const event: ExitEvent = {
        code: exitCode ?? null,
        signal: signal != null ? String(signal) : null,
      };
      for (const h of this.exitHandlers) h(event);
    });

    return proc.pid;
  }

  pause(): void {
    this.paused_ = true;
    if (this.adoptedSocket) {
      this.adoptedSocket.pause();
    }
    // ptyResumeSignal loop: paused_ flag stops forwarding at next iteration
  }

  resume(): void {
    this.paused_ = false;
    if (this.ptyResumeSignal) {
      const signal = this.ptyResumeSignal;
      this.ptyResumeSignal = null;
      signal();
    } else if (this.adoptedSocket) {
      this.adoptedSocket.resume();
    }
  }

  private startBunReadLoop(masterFd: number): void {
    const devPath = `/dev/fd/${masterFd}`;
    const loop = async () => {
      while (!this.killed && this.ptyProcess) {
        if (this.paused_) {
          await new Promise<void>((resolve) => {
            this.ptyResumeSignal = resolve;
          });
          this.ptyResumeSignal = null;
          if (this.killed) break;
        }
        try {
          const reader = Bun.file(devPath).stream().getReader();
          try {
            while (!this.paused_ && !this.killed) {
              const { done, value } = await reader.read();
              if (done) break;
              if (value && value.length > 0) {
                if (this.opts.captureRawIo) {
                  this.opts.logger.debug("pty.out", { bytes: Buffer.from(value).toString("hex") });
                }
                for (const h of this.dataHandlers) h(value);
              }
            }
          } finally {
            reader.cancel().catch(() => {});
          }
        } catch {
          if (!this.killed) await new Promise((r) => setTimeout(r, 10));
        }
      }
    };
    loop().catch(() => {});
  }

  getMasterFd(): number {
    if (this.adoptedFd_ !== null) return this.adoptedFd_;
    const fd = (this.ptyProcess as unknown as { _fd?: number })?._fd;
    if (typeof fd !== "number") throw new Error("node-pty _fd not accessible — check version pin");
    return fd;
  }

  write(bytes: Uint8Array): void {
    if (this.adoptedSocket) {
      this.adoptedSocket.write(bytes);
      return;
    }
    if (!this.ptyProcess) throw new AtError("TransportClosed");
    if (this.opts.captureRawIo) {
      this.opts.logger.debug("pty.in", { bytes: Buffer.from(bytes).toString("hex") });
    }
    this.ptyProcess.write(Buffer.from(bytes).toString("binary"));
  }

  sendInterrupt(): void {
    if (this.adoptedSocket) {
      this.adoptedSocket.write(INTERRUPT_KEY);
      return;
    }
    if (!this.ptyProcess) throw new AtError("TransportClosed");
    this.ptyProcess.write(INTERRUPT_KEY);
  }

  resize(cols: number, rows: number): void {
    if (this.adoptedFd_ !== null) {
      // Best-effort resize via node-pty's native binding on the inherited fd.
      try {
        const binding = (
          pty as unknown as {
            native?: { resize?: (fd: number, cols: number, rows: number) => void };
          }
        ).native;
        binding?.resize?.(this.adoptedFd_, cols, rows);
      } catch {
        /* not critical */
      }
      return;
    }
    this.ptyProcess?.resize(cols, rows);
  }

  async kill(): Promise<ExitEvent> {
    if (this.adoptedPid_ !== null) {
      if (process.platform !== "win32" && this.adoptedPid_ > 0) {
        try { process.kill(-this.adoptedPid_, "SIGTERM"); } catch { /* already dead */ }
      }
      try {
        process.kill(this.adoptedPid_, "SIGTERM");
      } catch {
        /* already dead */
      }
      return new Promise<ExitEvent>((resolve) => {
        const timer = setTimeout(() => {
          if (process.platform !== "win32" && this.adoptedPid_! > 0) {
            try { process.kill(-this.adoptedPid_!, "SIGKILL"); } catch { /* already dead */ }
          }
          try {
            process.kill(this.adoptedPid_!, "SIGKILL");
          } catch {
            /* already dead */
          }
          resolve({ code: null, signal: "SIGKILL" });
        }, this.opts.shutdownGraceMs);
        const handler = (event: ExitEvent) => {
          clearTimeout(timer);
          this.exitHandlers.delete(handler);
          resolve(event);
        };
        this.exitHandlers.add(handler);
      });
    }
    if (!this.ptyProcess || this.killed) {
      return { code: null, signal: null };
    }
    this.killed = true;

    return new Promise<ExitEvent>((resolve) => {
      const exitOnce = (event: ExitEvent) => {
        if (this.killTimer) {
          clearTimeout(this.killTimer);
          this.killTimer = null;
        }
        resolve(event);
      };
      this.exitHandlers.add(exitOnce);

      const pid = this.ptyProcess!.pid;
      if (process.platform !== "win32" && pid > 0) {
        try { process.kill(-pid, "SIGTERM"); } catch { /* already dead */ }
      }
      try {
        this.ptyProcess!.kill("SIGTERM");
      } catch {
        // already dead
      }

      this.killTimer = setTimeout(() => {
        this.opts.logger.warn("pty.kill: grace expired, sending SIGKILL");
        if (process.platform !== "win32" && pid > 0) {
          try { process.kill(-pid, "SIGKILL"); } catch { /* already dead */ }
        }
        try {
          this.ptyProcess?.kill("SIGKILL");
        } catch {
          // already dead
        }
      }, this.opts.shutdownGraceMs);
    });
  }

  onData(handler: DataHandler): () => void {
    this.dataHandlers.add(handler);
    return () => this.dataHandlers.delete(handler);
  }

  onExit(handler: ExitHandler): () => void {
    this.exitHandlers.add(handler);
    return () => this.exitHandlers.delete(handler);
  }

  private cleanup() {
    if (this.killTimer) {
      clearTimeout(this.killTimer);
      this.killTimer = null;
    }
    this.ptyProcess = null;
    // Unblock the read loop if it's waiting on a resume signal
    if (this.ptyResumeSignal) {
      const signal = this.ptyResumeSignal;
      this.ptyResumeSignal = null;
      signal();
    }
  }
}
