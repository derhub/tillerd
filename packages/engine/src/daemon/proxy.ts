import type {
  AgentSession,
  SessionStatus,
  ContentEvent,
  ExitEvent,
  AgentDefinition,
  SessionOptions,
} from "@athing/sdk";
import { AtError } from "@athing/sdk";
import { StatusMapper } from "../session/status";
import { TranscriptReader } from "../session/content";
import { SendQueue } from "../session/queue";
import type { DaemonFrame } from "@athing/sdk";
import type { DaemonClient, FrameHandler } from "./client";
import type { Logger } from "@athing/logger";
import { createLogger } from "@athing/logger";
import { randomBytes } from "node:crypto";

const DEFAULT_STARTUP_TIMEOUT = 30_000;
const DEFAULT_SHUTDOWN_GRACE = 5_000;
const DEFAULT_SEND_QUEUE_CAPACITY = 32;
const DEFAULT_COLS = 220;
const DEFAULT_ROWS = 50;

type DataHandler = (bytes: Uint8Array) => void;
type StatusHandler = (status: SessionStatus) => void;
type ContentHandler = (event: ContentEvent) => void;
type ErrorHandler = (err: AtError) => void;
type ExitHandler = (event: ExitEvent) => void;

export type ProxyMode = "spawn" | "subscribe";

export class AgentSessionProxy implements AgentSession {
  readonly sessionId: string;
  readonly token: string;

  private dataHandlers = new Set<DataHandler>();
  private statusHandlers = new Set<StatusHandler>();
  private contentHandlers = new Set<ContentHandler>();
  private errorHandlers = new Set<ErrorHandler>();
  private exitHandlers = new Set<ExitHandler>();

  private statusMapper: StatusMapper;
  private transcriptReader: TranscriptReader;
  private sendQueue: SendQueue;
  private logger: Logger;

  private dataBuf: Uint8Array[] = [];
  private dataBufBytes = 0;
  private exitEvent: ExitEvent | null = null;
  private started = false;
  private killed_ = false;
  private startupTimer: ReturnType<typeof setTimeout> | null = null;
  private idleTimer: ReturnType<typeof setTimeout> | null = null;
  private unsub: (() => void) | null = null;

  private readonly opts: Required<SessionOptions>;

  constructor(
    sessionId: string,
    private readonly adapter: AgentDefinition,
    opts: Required<SessionOptions>,
    private readonly client: DaemonClient,
    private readonly mode: ProxyMode,
    private readonly hooksSockPath: string,
  ) {
    this.sessionId = sessionId;
    this.token = randomBytes(32).toString("hex");
    this.logger = createLogger(sessionId);
    this.opts = opts;
    this.sendQueue = new SendQueue(opts.sendQueueCapacity);
    this.statusMapper = new StatusMapper();
    this.transcriptReader = new TranscriptReader(sessionId, adapter, opts.cwd, this.logger);

    this.statusMapper.onChange((status) => {
      this.cancelIdleTimer();
      if (status === "IDLE" || status === "WAITING_INPUT") {
        const queued = this.sendQueue.setReady(true);
        for (const text of queued) {
          this.sendText(text);
          this.sendQueue.setReady(false);
        }
        if (status === "IDLE" && queued.length === 0 && opts.idleTimeoutMs > 0) {
          this.startIdleTimer();
        }
      } else {
        this.sendQueue.setReady(false);
      }
      for (const h of this.statusHandlers) h(status);
    });

    this.transcriptReader.onContent((event) => {
      for (const h of this.contentHandlers) h(event);
    });

    this.transcriptReader.onError((err) => {
      for (const h of this.errorHandlers) h(err);
    });
  }

  start(): void {
    if (this.started || this.killed_) return;
    this.started = true;

    this.startupTimer = setTimeout(() => {
      const err = new AtError("Timeout", `Session ${this.sessionId} startup timed out`);
      for (const h of this.errorHandlers) h(err);
      void this.kill();
    }, this.opts.startupTimeoutMs);

    const handler: FrameHandler = (frame, body) => this.handleFrame(frame, body);
    this.unsub = this.client.subscribe(this.sessionId, handler);

    if (this.mode === "spawn") {
      const launchArgs = buildArgs(this.adapter, this.sessionId, this.opts.resume);
      this.client.send({
        type: "spawn",
        sessionId: this.sessionId,
        command: this.adapter.launch.command,
        args: launchArgs,
        flags: this.adapter.launch.flags,
        hookSocketPath: this.hooksSockPath,
        token: this.token,
        cols: this.opts.cols,
        rows: this.opts.rows,
        cwd: this.opts.cwd,
      });
    } else {
      this.client.send({ type: "subscribe", sessionId: this.sessionId });
    }
  }

  private handleFrame(frame: DaemonFrame, body: Buffer | null): void {
    switch (frame.type) {
      case "data": {
        const bytes = body ? new Uint8Array(body) : new Uint8Array(0);
        this.cancelStartupTimer();
        this.dataBuf.push(bytes);
        this.dataBufBytes += bytes.length;
        // Bounded replay: 512 KB
        while (this.dataBufBytes > 512 * 1024) {
          const dropped = this.dataBuf.shift()!;
          this.dataBufBytes -= dropped.length;
        }
        for (const h of this.dataHandlers) h(bytes);
        // Flow control: ack consumed bytes
        if (bytes.length > 0) {
          this.client.send({ type: "ack", sessionId: this.sessionId, bytes: bytes.length });
        }
        break;
      }

      case "spawn-ack": {
        this.cancelStartupTimer();
        break;
      }

      case "hook": {
        this.cancelStartupTimer();
        try {
          const hookEvent = this.adapter.parseHook(frame.payload);
          this.transcriptReader.onHook(hookEvent);
          this.statusMapper.apply(hookEvent);
        } catch (err) {
          this.logger.warn("proxy: parseHook failed", { err: String(err) });
        }
        break;
      }

      case "exit": {
        this.exitEvent = { code: frame.code, signal: frame.signal };
        this.transcriptReader.onExit();
        this.cancelStartupTimer();
        this.cancelIdleTimer();
        this.unsub?.();
        for (const h of this.exitHandlers) h(this.exitEvent);
        break;
      }

      case "error": {
        const err = new AtError(frame.code as import("@athing/sdk").ErrorKind, frame.message);
        for (const h of this.errorHandlers) h(err);
        break;
      }
    }
  }

  private sendText(text: string): void {
    const bytes = Buffer.from(`\x1b[200~${text}\x1b[201~\r`, "utf8");
    this.client.send({ type: "input", sessionId: this.sessionId }, bytes);
  }

  private cancelStartupTimer(): void {
    if (this.startupTimer) {
      clearTimeout(this.startupTimer);
      this.startupTimer = null;
    }
  }

  private startIdleTimer(): void {
    this.idleTimer = setTimeout(() => {
      this.idleTimer = null;
      const err = new AtError("Timeout", `Session ${this.sessionId} idle timeout exceeded`);
      for (const h of this.errorHandlers) h(err);
      void this.kill();
    }, this.opts.idleTimeoutMs);
  }

  private cancelIdleTimer(): void {
    if (this.idleTimer) {
      clearTimeout(this.idleTimer);
      this.idleTimer = null;
    }
  }

  send(text: string): void {
    if (this.sendQueue.isReady()) {
      this.sendQueue.setReady(false);
      this.sendText(text);
    } else {
      this.sendQueue.enqueue(text);
    }
  }

  input(bytes: Uint8Array): void {
    this.client.send({ type: "input", sessionId: this.sessionId }, Buffer.from(bytes));
  }

  interrupt(): void {
    this.client.send({ type: "interrupt", sessionId: this.sessionId });
  }

  resize(cols: number, rows: number): void {
    this.client.send({ type: "resize", sessionId: this.sessionId, cols, rows });
  }

  async kill(): Promise<ExitEvent> {
    this.killed_ = true;
    this.cancelStartupTimer();
    this.cancelIdleTimer();
    if (this.exitEvent) return this.exitEvent;
    // killed before proxy.start() ran — no spawn was sent, nothing to kill
    if (!this.started) {
      const event: ExitEvent = { code: null, signal: null };
      this.exitEvent = event;
      for (const h of this.exitHandlers) h(event);
      return event;
    }
    return new Promise<ExitEvent>((resolve) => {
      const handler: ExitHandler = (event) => {
        this.exitHandlers.delete(handler);
        resolve(event);
      };
      this.exitHandlers.add(handler);
      this.client.send({ type: "kill", sessionId: this.sessionId });
    });
  }

  onData(handler: DataHandler): () => void {
    for (const chunk of this.dataBuf) handler(chunk);
    this.dataHandlers.add(handler);
    return () => this.dataHandlers.delete(handler);
  }

  onStatus(handler: StatusHandler): () => void {
    this.statusHandlers.add(handler);
    return () => this.statusHandlers.delete(handler);
  }

  onContent(handler: ContentHandler): () => void {
    this.contentHandlers.add(handler);
    return () => this.contentHandlers.delete(handler);
  }

  onError(handler: ErrorHandler): () => void {
    this.errorHandlers.add(handler);
    return () => this.errorHandlers.delete(handler);
  }

  onExit(handler: ExitHandler): () => void {
    this.exitHandlers.add(handler);
    return () => this.exitHandlers.delete(handler);
  }
}

function buildArgs(adapter: AgentDefinition, sessionId: string, resume?: string): string[] {
  return adapter.launch.args
    .map((arg) => arg.replace("{id}", sessionId).replace("{resume}", resume ?? ""))
    .filter((a) => a !== "");
}

export function fillProxyOptions(opts?: SessionOptions): Required<SessionOptions> {
  return {
    cwd: opts?.cwd ?? process.cwd(),
    cols: opts?.cols ?? DEFAULT_COLS,
    rows: opts?.rows ?? DEFAULT_ROWS,
    resume: opts?.resume ?? "",
    startupTimeoutMs: opts?.startupTimeoutMs ?? DEFAULT_STARTUP_TIMEOUT,
    shutdownGraceMs: opts?.shutdownGraceMs ?? DEFAULT_SHUTDOWN_GRACE,
    idleTimeoutMs: opts?.idleTimeoutMs ?? 0,
    sendQueueCapacity: opts?.sendQueueCapacity ?? DEFAULT_SEND_QUEUE_CAPACITY,
    captureRawIo: opts?.captureRawIo ?? false,
  };
}
