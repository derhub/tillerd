import type { ExitEvent } from "@athing/sdk";
import { resolveSignal, signalCategoryToQualifier } from "@athing/sdk";
import { PtyTransport, type RawExitEvent } from "./pty-transport";
import { ReplayBuffer } from "./replay-buffer";
import { VtState, type SnapshotPayload } from "./vt-state";
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
  token: string;
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
  // Current terminal dimensions, updated on resize. The snapshot is built
  // on demand at these dims by replaying the ring buffer — the daemon does
  // not maintain a live virtual terminal.
  private curCols: number;
  private curRows: number;
  private subscribers = new Map<unknown, Subscription>();
  private exitCallbacks = new Set<ExitCallback>();
  private logger: Logger;
  private killedByUser = false;

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
      this.token = meta!.token;
      this.cwd = meta!.cwd;
      this.cols = meta!.cols;
      this.rows = meta!.rows;
      this.pid_ = meta!.pid;
      this.transport = transport!;
      this.logger = createLogger(sessionId!);
      this.curCols = meta!.cols;
      this.curRows = meta!.rows;
      if (meta!.replayBuffer.length > 0) {
        this.replayBuffer.push(meta!.replayBuffer);
      }
      this.wireTransport();
      return;
    }

    const opts = optsOrSentinel;
    this.sessionId = opts.sessionId;
    this.token = opts.token;
    this.cwd = opts.cwd;
    this.cols = opts.cols;
    this.rows = opts.rows;
    this.logger = createLogger(opts.sessionId);
    this.curCols = opts.cols;
    this.curRows = opts.rows;

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
    this.wireTransport();
    this.pid_ = this.transport.spawn();
    return this.pid_;
  }

  markKilledByUser(): void {
    this.killedByUser = true;
  }

  private translateExit(raw: RawExitEvent): ExitEvent {
    if (this.killedByUser) {
      return { qualifier: "stopped-by-request", raw };
    }
    if (!raw.signal) {
      const qualifier = raw.code === 0 ? "ok" : "error";
      this.logger.info("exit.qualifier", { qualifier, killedByUser: false, code: raw.code, signal: null });
      return { qualifier, raw };
    }
    const resolved = resolveSignal(raw.signal);
    const rawWithSignal: import("@athing/sdk").ExitEventRaw = {
      ...raw,
      signalName: resolved.name === "unknown" ? undefined : resolved.name,
      signalMeaning: resolved.name === "unknown" ? undefined : (resolved as { meaning: string }).meaning,
      signalCategory: resolved.name === "unknown" ? undefined : (resolved as import("@athing/sdk").SignalInfo).category,
    };
    if (resolved.name === "unknown") {
      this.logger.info("exit.qualifier", { qualifier: "unknown", killedByUser: false, signal: raw.signal });
      return { qualifier: "unknown", raw: rawWithSignal };
    }
    // SIGHUP specifically maps to hangup (graceful-termination category is shared with SIGINT/SIGQUIT)
    if (resolved.name === "SIGHUP") {
      this.logger.info("exit.qualifier", { qualifier: "hangup", killedByUser: false, signal: resolved.name, category: (resolved as { category: string }).category });
      return { qualifier: "hangup", raw: rawWithSignal };
    }
    const qualifier = signalCategoryToQualifier((resolved as { category: Parameters<typeof signalCategoryToQualifier>[0] }).category, false);
    this.logger.info("exit.qualifier", { qualifier, killedByUser: false, signal: resolved.name, category: (resolved as { category: string }).category });
    return { qualifier, raw: rawWithSignal };
  }

  private wireTransport(): void {
    this.transport.onData((bytes) => {
      // Hot path: raw bytes only — ring buffer + forward. No parsing.
      this.replayBuffer.push(bytes);
      this.emitData(bytes);
    });
    this.transport.onExit((raw: RawExitEvent) => {
      const event = this.translateExit(raw);
      for (const cb of this.exitCallbacks) cb(event);
    });
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
    this.curCols = cols;
    this.curRows = rows;
    this.transport.resize(cols, rows);
  }

  // Build a terminal snapshot on demand by replaying the ring buffer through a
  // fresh VT parser at the current dimensions. The daemon keeps no live virtual
  // terminal — fidelity is bounded by the ring-buffer window, and any miss
  // self-heals as live output repaints the screen.
  getSnapshot(): SnapshotPayload {
    const vt = new VtState(this.curRows, this.curCols);
    vt.feed(this.getReplayBytes());
    const snap = vt.getSnapshot();
    vt.dispose();
    this.logger.info("snapshot.generate", { sessionId: this.sessionId, rows: snap.rows, cols: snap.cols });
    return snap;
  }

  async kill(): Promise<ExitEvent> {
    const raw = await this.transport.kill();
    return this.translateExit(raw);
  }

  getMasterFd(): number {
    return this.transport.getMasterFd();
  }
}
