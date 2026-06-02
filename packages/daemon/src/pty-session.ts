import type { ExitEvent } from "@athing/sdk";
import { PtyTransport } from "./pty-transport";
import { ReplayBuffer } from "./replay-buffer";
import type { Logger } from "@athing/logger";
import { createLogger } from "@athing/logger";

const DEFAULT_SHUTDOWN_GRACE = 5_000;
const INITIAL_CREDIT = 65_536;

export interface PtySessionOptions {
  sessionId: string;
  token: string;
  command: string;
  args: string[];
  flags: string[];
  hookSocketPath: string;
  cols: number;
  rows: number;
  cwd: string;
}

type DataCallback = (bytes: Uint8Array) => void;
type ExitCallback = (event: ExitEvent) => void;

interface Subscription {
  onData: DataCallback;
  credit: number;
  paused: boolean;
}

export interface AdoptedSessionMeta {
  replayBuffer: Uint8Array;
  cwd: string;
  cols: number;
  rows: number;
  pid: number;
}

// Sentinel used by the static factory to signal adopted-transport mode.
const ADOPT = Symbol("adopt");

export class PtySession {
  readonly sessionId: string;
  readonly token: string;
  readonly cwd: string;
  readonly cols: number;
  readonly rows: number;

  private pid_: number | null = null;
  private transport: PtyTransport;
  private replayBuffer = new ReplayBuffer();
  private subscribers = new Map<unknown, Subscription>();
  private exitCallbacks = new Set<ExitCallback>();
  private logger: Logger;

  get pid(): number {
    if (this.pid_ === null) throw new Error("session not started");
    return this.pid_;
  }

  constructor(opts: PtySessionOptions);
  constructor(
    sentinel: typeof ADOPT,
    sessionId: string,
    transport: PtyTransport,
    meta: AdoptedSessionMeta,
  );
  constructor(
    optsOrSentinel: PtySessionOptions | typeof ADOPT,
    sessionId?: string,
    transport?: PtyTransport,
    meta?: AdoptedSessionMeta,
  ) {
    if (optsOrSentinel === ADOPT) {
      this.sessionId = sessionId!;
      this.token = "";
      this.cwd = meta!.cwd;
      this.cols = meta!.cols;
      this.rows = meta!.rows;
      this.pid_ = meta!.pid;
      this.transport = transport!;
      this.logger = createLogger(sessionId!);
      if (meta!.replayBuffer.length > 0) this.replayBuffer.push(meta!.replayBuffer);
      // Wire transport events immediately — no spawn() needed.
      this.transport.onData((bytes) => {
        this.replayBuffer.push(bytes);
        this.emitData(bytes);
      });
      this.transport.onExit((event) => {
        for (const cb of this.exitCallbacks) cb(event);
      });
      return;
    }

    const opts = optsOrSentinel;
    this.sessionId = opts.sessionId;
    this.token = opts.token;
    this.cwd = opts.cwd;
    this.cols = opts.cols;
    this.rows = opts.rows;
    this.logger = createLogger(opts.sessionId);

    const launchArgs = [...opts.args, ...opts.flags].filter((a) => a !== "");

    this.transport = new PtyTransport({
      command: opts.command,
      args: launchArgs,
      cwd: opts.cwd,
      env: {
        ATHING_BRIDGE_URL: opts.hookSocketPath,
        ATHING_SESSION_ID: opts.sessionId,
        ATHING_SESSION_TOKEN: opts.token,
      },
      cols: opts.cols,
      rows: opts.rows,
      logger: this.logger,
      shutdownGraceMs: DEFAULT_SHUTDOWN_GRACE,
    });
  }

  /**
   * Create a session wrapping an already-adopted PTY transport (successor daemon use-case).
   * The transport must have been created via PtyTransport.adoptFromFd().
   */
  static fromAdoptedTransport(
    sessionId: string,
    transport: PtyTransport,
    meta: AdoptedSessionMeta,
  ): PtySession {
    return new PtySession(ADOPT, sessionId, transport, meta);
  }

  start(): number {
    this.transport.onData((bytes) => {
      this.replayBuffer.push(bytes);
      this.emitData(bytes);
    });

    this.transport.onExit((event: ExitEvent) => {
      for (const cb of this.exitCallbacks) cb(event);
    });

    this.pid_ = this.transport.spawn();
    return this.pid_;
  }

  addSubscriber(key: unknown, onData: DataCallback, initialCredit = INITIAL_CREDIT): void {
    this.subscribers.set(key, { onData, credit: initialCredit, paused: false });
  }

  removeSubscriber(key: unknown): void {
    this.subscribers.delete(key);
    if (this.subscribers.size === 0) this.transport.resume();
  }

  addCredit(key: unknown, bytes: number): void {
    const sub = this.subscribers.get(key);
    if (!sub) return;
    const wasPaused = sub.paused;
    sub.credit += bytes;
    if (sub.paused && sub.credit > 0) {
      sub.paused = false;
    }
    // Resume PTY fd as soon as any subscriber has credit again (per D3).
    if (wasPaused && !sub.paused) {
      this.transport.resume();
    }
  }

  emitToSubscribers(fn: (key: unknown) => void): void {
    for (const key of this.subscribers.keys()) fn(key);
  }

  private emitData(bytes: Uint8Array): void {
    for (const sub of this.subscribers.values()) {
      if (sub.paused) continue;
      sub.credit -= bytes.length;
      if (sub.credit <= 0) {
        sub.credit = 0;
        sub.paused = true;
      }
      try {
        sub.onData(bytes);
      } catch {
        // subscriber gone
      }
    }
    // Pause PTY fd only when NO subscriber has remaining credit (per D3: task 5.4).
    if (this.subscribers.size > 0 && !this.anyActive()) {
      this.transport.pause();
    }
  }

  private anyActive(): boolean {
    for (const sub of this.subscribers.values()) {
      if (!sub.paused) return true;
    }
    return false;
  }

  getReplayBytes(): Uint8Array {
    const chunks = this.replayBuffer.snapshot();
    const total = chunks.reduce((n, c) => n + c.length, 0);
    const buf = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) {
      buf.set(chunk, offset);
      offset += chunk.length;
    }
    return buf;
  }

  onExit(cb: ExitCallback): void {
    this.exitCallbacks.add(cb);
  }

  onExitOnce(cb: ExitCallback): void {
    const wrapper = (event: ExitEvent) => {
      this.exitCallbacks.delete(wrapper);
      cb(event);
    };
    this.exitCallbacks.add(wrapper);
  }

  write(bytes: Uint8Array): void {
    this.transport.write(bytes);
  }

  interrupt(): void {
    this.transport.sendInterrupt();
  }

  resize(cols: number, rows: number): void {
    this.transport.resize(cols, rows);
  }

  async kill(): Promise<ExitEvent> {
    return this.transport.kill();
  }

  getMasterFd(): number {
    return this.transport.getMasterFd();
  }
}
