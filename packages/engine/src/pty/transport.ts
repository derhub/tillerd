import * as pty from "node-pty";
import type { IPty } from "node-pty";
import type { ExitEvent } from "@athing/sdk";
import { AtError } from "@athing/sdk";
import type { Logger } from "../logger";
import { resolveBinary } from "./resolve";

const BRACKETED_PASTE_START = "\x1b[200~";
const BRACKETED_PASTE_END = "\x1b[201~";
const INTERRUPT_KEY = "\x1b";
const SUBMIT_KEY = "\r";

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
  private dataHandlers = new Set<DataHandler>();
  private exitHandlers = new Set<ExitHandler>();
  private killed = false;
  private killTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(private readonly opts: PtyTransportOptions) {}

  spawn(): void {
    const binary = resolveBinary(this.opts.command);
    this.opts.logger.info("spawning pty", { binary, args: this.opts.args });

    const proc = pty.spawn(
      process.env["SHELL"] ?? "/bin/sh",
      ["-lc", `exec ${binary} ${this.opts.args.join(" ")}`],
      {
        name: "xterm-256color",
        cols: this.opts.cols ?? DEFAULT_COLS,
        rows: this.opts.rows ?? DEFAULT_ROWS,
        cwd: this.opts.cwd,
        env: { ...process.env, ...this.opts.env } as Record<string, string>,
      },
    );

    proc.onData((data) => {
      const bytes = Buffer.from(data, "binary");
      if (this.opts.captureRawIo) {
        this.opts.logger.debug("pty.out", { bytes: bytes.toString("hex") });
      }
      for (const h of this.dataHandlers) h(bytes);
    });

    proc.onExit(({ exitCode, signal }) => {
      this.opts.logger.info("pty.exit", { exitCode, signal });
      this.cleanup();
      const event: ExitEvent = {
        code: exitCode ?? null,
        signal: signal != null ? String(signal) : null,
      };
      for (const h of this.exitHandlers) h(event);
    });

    this.ptyProcess = proc;
  }

  write(bytes: Uint8Array): void {
    if (!this.ptyProcess) throw new AtError("TransportClosed");
    if (this.opts.captureRawIo) {
      this.opts.logger.debug("pty.in", { bytes: Buffer.from(bytes).toString("hex") });
    }
    this.ptyProcess.write(Buffer.from(bytes).toString("binary"));
  }

  sendPrompt(text: string): void {
    if (!this.ptyProcess) throw new AtError("TransportClosed");
    const escaped = `${BRACKETED_PASTE_START}${text}${BRACKETED_PASTE_END}${SUBMIT_KEY}`;
    this.ptyProcess.write(escaped);
  }

  sendInterrupt(): void {
    if (!this.ptyProcess) throw new AtError("TransportClosed");
    this.ptyProcess.write(INTERRUPT_KEY);
  }

  resize(cols: number, rows: number): void {
    this.ptyProcess?.resize(cols, rows);
  }

  async kill(): Promise<ExitEvent> {
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

      try {
        this.ptyProcess!.kill("SIGTERM");
      } catch {
        // already dead
      }

      this.killTimer = setTimeout(() => {
        this.opts.logger.warn("pty.kill: grace expired, sending SIGKILL");
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
  }
}
