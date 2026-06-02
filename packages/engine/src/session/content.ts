import * as fs from "node:fs";
import type { HookEvent, ContentEvent, AgentDefinition } from "@athing/sdk";
import { AtError } from "@athing/sdk";
import type { Logger } from "@athing/logger";

type ContentHandler = (event: ContentEvent) => void;
type ErrorHandler = (err: AtError) => void;

export class TranscriptReader {
  private offset = 0;
  private lastSize = 0;
  private transcriptAbsent = false;
  private handlers = new Set<ContentHandler>();
  private errorHandlers = new Set<ErrorHandler>();

  constructor(
    private readonly sessionId: string,
    private readonly adapter: AgentDefinition,
    private readonly cwd: string,
    private readonly logger: Logger,
  ) {}

  onHook(event: HookEvent): void {
    if (event.type === "PostToolUse" || event.type === "Stop") {
      this.readDelta();
    }
  }

  onExit(): void {
    this.readDelta();
  }

  private readDelta(): void {
    const filePath = this.adapter.transcriptPath(this.sessionId, this.cwd);

    let stat: fs.Stats;
    try {
      stat = fs.statSync(filePath);
      this.transcriptAbsent = false;
    } catch {
      if (!this.transcriptAbsent) {
        this.transcriptAbsent = true;
        this.emitError(new AtError("TranscriptUnavailable", `Transcript not present: ${filePath}`));
      }
      return;
    }

    if (stat.size < this.offset) {
      this.logger.warn("transcript: truncation detected, resetting", { path: filePath });
      this.offset = 0;
      this.lastSize = 0;
    }

    if (stat.size === this.lastSize) return;

    let fd: number;
    try {
      fd = fs.openSync(filePath, "r");
    } catch (err) {
      this.emitError(new AtError("TranscriptUnavailable", String(err)));
      return;
    }

    try {
      const toRead = stat.size - this.offset;
      if (toRead <= 0) return;

      const buf = Buffer.alloc(toRead);
      const bytesRead = fs.readSync(fd, buf, 0, toRead, this.offset);
      this.offset += bytesRead;
      this.lastSize = stat.size;

      const chunk = buf.subarray(0, bytesRead).toString("utf8");
      for (const line of chunk.split("\n")) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        try {
          const content = this.adapter.parseTranscriptEntry(trimmed);
          if (content) {
            for (const h of this.handlers) h(content);
          }
        } catch (err) {
          this.logger.debug("transcript: parse error on line", { err: String(err) });
        }
      }
    } finally {
      fs.closeSync(fd);
    }
  }

  private emitError(err: AtError): void {
    for (const h of this.errorHandlers) h(err);
  }

  onContent(handler: ContentHandler): () => void {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }

  onError(handler: ErrorHandler): () => void {
    this.errorHandlers.add(handler);
    return () => this.errorHandlers.delete(handler);
  }
}
