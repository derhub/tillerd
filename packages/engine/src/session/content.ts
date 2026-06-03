import type { HookEvent, ContentEvent, AgentDefinition, FileSource, Logger } from "@athing/sdk";
import { AtError } from "@athing/sdk";

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
    private readonly fileSource: FileSource,
    private readonly agentHome: string,
  ) {}

  onHook(event: HookEvent): void {
    if (event.type === "PostToolUse" || event.type === "Stop") {
      void this.readDelta();
    }
  }

  onExit(): void {
    void this.readDelta();
  }

  private async readDelta(): Promise<void> {
    const filePath = this.adapter.transcriptPath(this.sessionId, this.cwd, this.agentHome);

    const size = await this.fileSource.size(filePath);
    if (size === null) {
      if (!this.transcriptAbsent) {
        this.transcriptAbsent = true;
        this.emitError(new AtError("TranscriptUnavailable", `Transcript not present: ${filePath}`));
      }
      return;
    }
    this.transcriptAbsent = false;

    if (size < this.offset) {
      this.logger.warn("transcript: truncation detected, resetting", { path: filePath });
      this.offset = 0;
      this.lastSize = 0;
    }

    if (size === this.lastSize) return;

    const toRead = size - this.offset;
    if (toRead <= 0) return;

    let bytes: Uint8Array;
    try {
      bytes = await this.fileSource.read(filePath, this.offset, toRead);
    } catch (err) {
      this.emitError(new AtError("TranscriptUnavailable", String(err)));
      return;
    }

    this.offset += bytes.length;
    this.lastSize = size;

    const chunk = new TextDecoder().decode(bytes);
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
