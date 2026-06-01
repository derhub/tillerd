import type {
  AgentSession,
  SessionStatus,
  ContentEvent,
  ExitEvent,
  AgentDefinition,
  SessionOptions,
} from "@athing/sdk";
import { AtError } from "@athing/sdk";
import { PtyTransport } from "../pty/transport";
import { StatusMapper } from "./status";
import { TranscriptReader } from "./content";
import { SendQueue } from "./queue";
import type { HookDispatcher } from "../ingress/dispatcher";
import type { Logger } from "../logger";
import { createLogger } from "../logger";
import { randomBytes } from "node:crypto";

const DEFAULT_STARTUP_TIMEOUT = 30_000;
const DEFAULT_SHUTDOWN_GRACE = 5_000;
const DEFAULT_SEND_QUEUE_CAPACITY = 32;
const DEFAULT_COLS = 220;
const DEFAULT_ROWS = 50;
const AUTH_SCAN_LIMIT = 8192;
const DATA_BUF_LIMIT = 512 * 1024; // 512 KB bounded replay buffer per session
const NOT_AUTH_PATTERNS = [
  "claude.ai/login",
  "run claude login",
  "not logged in",
  "login to claude",
  "anthropic.com/login",
];

type DataHandler = (bytes: Uint8Array) => void;
type StatusHandler = (status: SessionStatus) => void;
type ContentHandler = (event: ContentEvent) => void;
type ErrorHandler = (err: AtError) => void;
type ExitHandler = (event: ExitEvent) => void;

export class AgentSessionImpl implements AgentSession {
  readonly sessionId: string;
  readonly token: string;

  private dataHandlers = new Set<DataHandler>();
  private statusHandlers = new Set<StatusHandler>();
  private contentHandlers = new Set<ContentHandler>();
  private errorHandlers = new Set<ErrorHandler>();
  private exitHandlers = new Set<ExitHandler>();

  private transport: PtyTransport;
  private statusMapper: StatusMapper;
  private transcriptReader: TranscriptReader;
  private sendQueue: SendQueue;
  private logger: Logger;
  private startupTimer: ReturnType<typeof setTimeout> | null = null;
  private idleTimer: ReturnType<typeof setTimeout> | null = null;
  private killed_ = false;
  private authBuf = "";
  private authChecked = false;
  private dataBuf: Uint8Array[] = [];
  private dataBufBytes = 0;
  private exitEvent: ExitEvent | null = null;
  private started = false;

  constructor(
    sessionId: string,
    private readonly adapter: AgentDefinition,
    private readonly opts: Required<SessionOptions>,
    private readonly dispatcher: HookDispatcher,
    bridgeUrl: string,
  ) {
    this.sessionId = sessionId;
    this.token = randomBytes(32).toString("hex");
    this.logger = createLogger(sessionId);
    this.sendQueue = new SendQueue(opts.sendQueueCapacity);
    this.statusMapper = new StatusMapper();
    this.transcriptReader = new TranscriptReader(sessionId, adapter, opts.cwd, this.logger);

    const launchArgs = buildArgs(adapter, sessionId, opts.resume);

    this.transport = new PtyTransport({
      command: adapter.launch.command,
      args: launchArgs,
      cwd: opts.cwd,
      env: {
        ATHING_BRIDGE_URL: bridgeUrl,
        ATHING_SESSION_ID: sessionId,
        ATHING_SESSION_TOKEN: this.token,
      },
      cols: opts.cols,
      rows: opts.rows,
      logger: this.logger,
      captureRawIo: opts.captureRawIo,
      shutdownGraceMs: opts.shutdownGraceMs,
    });

    this.wireTransport();
  }

  private wireTransport(): void {
    this.transport.onData((bytes) => {
      if (!this.authChecked) {
        this.authBuf += Buffer.from(bytes).toString("utf8");
        if (this.authBuf.length >= AUTH_SCAN_LIMIT) {
          this.authChecked = true;
          this.authBuf = "";
        } else {
          const lower = this.authBuf.toLowerCase();
          if (NOT_AUTH_PATTERNS.some((p) => lower.includes(p))) {
            this.authChecked = true;
            this.authBuf = "";
            this.cancelStartupTimer();
            const err = new AtError("NotAuthenticated", "Agent requires authentication");
            for (const h of this.errorHandlers) h(err);
            void this.kill();
          }
        }
      }
      // Maintain bounded replay buffer; drop oldest when over limit (logged)
      this.dataBuf.push(bytes);
      this.dataBufBytes += bytes.length;
      while (this.dataBufBytes > DATA_BUF_LIMIT) {
        const dropped = this.dataBuf.shift()!;
        this.dataBufBytes -= dropped.length;
        this.logger.warn("data buffer overflow, dropping oldest chunk", {
          droppedBytes: dropped.length,
          bufferBytes: this.dataBufBytes,
        });
      }

      for (const h of this.dataHandlers) h(bytes);
    });

    this.transport.onExit((event) => {
      this.exitEvent = event;
      this.transcriptReader.onExit();
      this.cancelStartupTimer();
      this.cancelIdleTimer();
      this.dispatcher.unregister(this.sessionId);
      for (const h of this.exitHandlers) h(event);
    });

    this.statusMapper.onChange((status) => {
      this.cancelIdleTimer();
      if (status === "IDLE" || status === "WAITING_INPUT") {
        const queued = this.sendQueue.setReady(true);
        for (const text of queued) {
          this.transport.sendPrompt(text);
          this.sendQueue.setReady(false);
        }
        if (status === "IDLE" && queued.length === 0 && this.opts.idleTimeoutMs > 0) {
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

    this.dispatcher.register(this.sessionId, this.token, this.adapter, (event) => {
      this.transcriptReader.onHook(event);
      this.statusMapper.apply(event);
    });

    this.startupTimer = setTimeout(() => {
      const err = new AtError("Timeout", `Session ${this.sessionId} startup timed out`);
      for (const h of this.errorHandlers) h(err);
      void this.kill();
    }, this.opts.startupTimeoutMs);

    try {
      this.transport.spawn();
    } catch (err) {
      this.cancelStartupTimer();
      const atErr = err instanceof AtError ? err : new AtError("SpawnFailed", String(err));
      for (const h of this.errorHandlers) h(atErr);
    }
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
      this.transport.sendPrompt(text);
    } else {
      this.sendQueue.enqueue(text);
    }
  }

  input(bytes: Uint8Array): void {
    this.transport.write(bytes);
  }

  interrupt(): void {
    this.transport.sendInterrupt();
  }

  resize(cols: number, rows: number): void {
    this.transport.resize(cols, rows);
  }

  async kill(): Promise<ExitEvent> {
    this.killed_ = true;
    this.cancelStartupTimer();
    this.cancelIdleTimer();
    if (this.exitEvent) return this.exitEvent;
    return this.transport.kill();
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
    .concat(adapter.launch.flags)
    .filter((a) => a !== "");
}

export function fillOptions(opts?: SessionOptions): Required<SessionOptions> {
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
