import * as pty from "node-pty";
import type { IPty } from "node-pty";
import * as net from "node:net";
import { AtError } from "@athing/sdk";
import type { Logger } from "@athing/logger";
import { resolveCommand } from "./resolve";
import { captureDescendants, killPids } from "./process-tree";

export interface RawExitEvent {
  code: number | null;
  signal: string | null;
}

const DEFAULT_COLS = 220;
const DEFAULT_ROWS = 50;

// The daemon runs on Node: node-pty's native onData/write deliver PTY IO
// directly. (It is not run under Bun, whose fd model breaks node-pty IO.)

/**
 * Compute the child environment: a generic terminal base (PATH/HOME/TERM/...)
 * derived from the daemon's startup-installed login-shell env, with the
 * caller-supplied environment merged on top — caller entries win. Passing the
 * full process.env can break posix_spawnp (E2BIG, null bytes), so the base is a
 * minimal allowlist.
 */
export function buildChildEnv(
  callerEnv: Record<string, string>,
  shellFallback: string,
): Record<string, string> {
  return {
    PATH: process.env["PATH"] ?? "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
    HOME: process.env["HOME"] ?? "",
    USER: process.env["USER"] ?? "",
    LOGNAME: process.env["LOGNAME"] ?? process.env["USER"] ?? "",
    SHELL: process.env["SHELL"] ?? shellFallback,
    LANG: process.env["LANG"] ?? "en_US.UTF-8",
    TERM: "xterm-256color",
    COLORTERM: "truecolor",
    ...(process.env["SSH_AUTH_SOCK"] ? { SSH_AUTH_SOCK: process.env["SSH_AUTH_SOCK"] } : {}),
    ...callerEnv,
  };
}

export interface PtyTransportOptions {
  command?: string;
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
type ExitHandler = (event: RawExitEvent) => void;

export class PtyTransport {
  private ptyProcess: IPty | null = null;
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
    const binary = resolveCommand(this.opts.command);
    this.opts.logger.info("spawning pty", { binary, args: this.opts.args });

    const safeEnv = buildChildEnv(this.opts.env, binary);

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
      // Raw bytes (Buffer) so output passes through untouched (raw-bytes-end-to-end).
      encoding: null,
    };

    // When no args are passed the caller wants an interactive terminal (raw shell).
    // Spawn the binary directly so it runs as an interactive (login) session rather
    // than nested inside a second shell via -lc, which causes posix_spawnp failures
    // on some systems and prevents proper interactive-shell initialisation.
    // Spawn the resolved command directly. A bare login shell (no command, no
    // args) gets `-l` so it initialises as a login session; everything else is
    // launched verbatim — no wrapper shell, so no prompt or echo leaks.
    const isLoginShell =
      !this.opts.command &&
      this.opts.args.length === 0 &&
      /\/(sh|bash|zsh|fish|dash|csh|tcsh|ksh)$/.test(binary);
    const proc = pty.spawn(binary, isLoginShell ? ["-l"] : this.opts.args, ptyOpts);

    this.ptyProcess = proc;

    // node-pty's native onData delivers raw output (encoding:null).
    proc.onData((data: string | Buffer) => {
      if (this.paused_) return;
      const bytes = typeof data === "string" ? Buffer.from(data, "binary") : Buffer.from(data);
      if (this.opts.captureRawIo) {
        this.opts.logger.debug("pty.out", { bytes: bytes.toString("hex") });
      }
      for (const h of this.dataHandlers) h(new Uint8Array(bytes));
    });

    proc.onExit(({ exitCode, signal }) => {
      this.opts.logger.info("pty.exit", { exitCode, signal });
      this.cleanup();
      const event: RawExitEvent = {
        code: exitCode ?? null,
        // node-pty reports signal 0 for a normal (signal-free) exit; treat it as
        // no signal so the qualifier resolves to ok/error, not unknown.
        signal: signal != null && signal !== 0 ? String(signal) : null,
      };
      for (const h of this.exitHandlers) h(event);
    });

    return proc.pid;
  }

  pause(): void {
    this.paused_ = true;
    if (this.adoptedSocket) {
      this.adoptedSocket.pause();
    } else if (this.ptyProcess) {
      // Stop node-pty's stream so paused output is buffered, not dropped.
      this.ptyProcess.pause();
    }
  }

  resume(): void {
    this.paused_ = false;
    if (this.adoptedSocket) {
      this.adoptedSocket.resume();
    } else if (this.ptyProcess) {
      this.ptyProcess.resume();
    }
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

  async kill(): Promise<RawExitEvent> {
    if (this.adoptedPid_ !== null) {
      // Snapshot detached descendants before any signal lands; once the leader
      // dies they reparent to init and become unreachable by parent chain.
      const treePids = captureDescendants(this.adoptedPid_);
      if (process.platform !== "win32" && this.adoptedPid_ > 0) {
        try {
          process.kill(-this.adoptedPid_, "SIGTERM");
        } catch {
          /* already dead */
        }
      }
      try {
        process.kill(this.adoptedPid_, "SIGTERM");
      } catch {
        /* already dead */
      }
      killPids(treePids, "SIGTERM");
      return new Promise<RawExitEvent>((resolve) => {
        const timer = setTimeout(() => {
          if (process.platform !== "win32" && this.adoptedPid_! > 0) {
            try {
              process.kill(-this.adoptedPid_!, "SIGKILL");
            } catch {
              /* already dead */
            }
          }
          try {
            process.kill(this.adoptedPid_!, "SIGKILL");
          } catch {
            /* already dead */
          }
          killPids(treePids, "SIGKILL");
          resolve({ code: null, signal: "SIGKILL" });
        }, this.opts.shutdownGraceMs);
        const handler = (event: RawExitEvent) => {
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

    return new Promise<RawExitEvent>((resolve) => {
      const exitOnce = (event: RawExitEvent) => {
        if (this.killTimer) {
          clearTimeout(this.killTimer);
          this.killTimer = null;
        }
        resolve(event);
      };
      this.exitHandlers.add(exitOnce);

      const pid = this.ptyProcess!.pid;
      // Snapshot detached descendants before any signal lands; once the leader
      // dies they reparent to init and become unreachable by parent chain.
      const treePids = captureDescendants(pid);
      if (process.platform !== "win32" && pid > 0) {
        try {
          process.kill(-pid, "SIGTERM");
        } catch {
          /* already dead */
        }
      }
      try {
        this.ptyProcess!.kill("SIGTERM");
      } catch {
        // already dead
      }
      killPids(treePids, "SIGTERM");

      this.killTimer = setTimeout(() => {
        this.opts.logger.warn("pty.kill: grace expired, sending SIGKILL");
        if (process.platform !== "win32" && pid > 0) {
          try {
            process.kill(-pid, "SIGKILL");
          } catch {
            /* already dead */
          }
        }
        try {
          this.ptyProcess?.kill("SIGKILL");
        } catch {
          // already dead
        }
        killPids(treePids, "SIGKILL");
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
  }
}
