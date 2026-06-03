import { test, expect, describe } from "bun:test";
import type { AgentDefinition, ContentEvent, FileSource, HookEvent, Logger } from "@athing/sdk";
import { TranscriptReader } from "../src/session/content";

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

class FakeFileSource implements FileSource {
  constructor(
    private sizeVal: number | null,
    private bytes: Uint8Array = new Uint8Array(0),
  ) {}
  async size(): Promise<number | null> {
    return this.sizeVal;
  }
  async read(_path: string, offset: number, length: number): Promise<Uint8Array> {
    return this.bytes.slice(offset, offset + length);
  }
}

function spyLogger() {
  const debugCalls: string[] = [];
  const warnCalls: string[] = [];
  const logger: Logger = {
    debug: (m) => debugCalls.push(m),
    info: () => {},
    warn: (m) => warnCalls.push(m),
    error: () => {},
  };
  return { logger, debugCalls, warnCalls };
}

function adapterWith(
  parseTranscriptEntry: AgentDefinition["parseTranscriptEntry"],
): AgentDefinition {
  return {
    name: "mock",
    launch: { command: "mock", args: [], flags: [] },
    interruptSequence: "\x1b",
    binaryResolution: { overrideEnvVar: "MOCK_BIN", binaryName: "mock", commonLocations: [] },
    cliVersionRange: "*",
    parseHook: () => ({ sessionId: "s", type: "SessionStart", payload: {} }),
    transcriptPath: () => "/virtual/transcript.jsonl",
    parseTranscriptEntry,
  };
}

const postToolUse: HookEvent = { sessionId: "s", type: "PostToolUse", payload: {} };

describe("TranscriptReader — injected FileSource", () => {
  test("emits content derived from the file-read contract on hook", async () => {
    const line = '{"x":1}';
    const fileSource = new FakeFileSource(line.length, new TextEncoder().encode(line));
    const { logger } = spyLogger();
    const reader = new TranscriptReader(
      "s",
      adapterWith(() => ({ kind: "tool_use" }) as unknown as ContentEvent),
      "/cwd",
      logger,
      fileSource,
      "/virtual-home",
    );

    const events: ContentEvent[] = [];
    reader.onContent((e) => events.push(e));
    reader.onHook(postToolUse);
    await sleep(10);

    expect(events).toHaveLength(1);
  });

  test("absent transcript reports a typed TranscriptUnavailable error", async () => {
    const fileSource = new FakeFileSource(null);
    const { logger } = spyLogger();
    const reader = new TranscriptReader(
      "s",
      adapterWith(() => null),
      "/cwd",
      logger,
      fileSource,
      "/virtual-home",
    );

    const kinds: string[] = [];
    reader.onError((e) => kinds.push(e.kind));
    reader.onExit();
    await sleep(10);

    expect(kinds).toContain("TranscriptUnavailable");
  });

  test("routes parse diagnostics through the injected logger", async () => {
    const line = "not-json";
    const fileSource = new FakeFileSource(line.length, new TextEncoder().encode(line));
    const { logger, debugCalls } = spyLogger();
    const reader = new TranscriptReader(
      "s",
      adapterWith(() => {
        throw new Error("bad line");
      }),
      "/cwd",
      logger,
      fileSource,
      "/virtual-home",
    );

    reader.onContent(() => {});
    reader.onHook(postToolUse);
    await sleep(10);

    expect(debugCalls.length).toBeGreaterThan(0);
  });

  test("does not read the filesystem when the contract reports a size", async () => {
    // A non-null size with empty delta must not throw or touch node:fs — pure contract use.
    const fileSource = new FakeFileSource(0);
    const { logger } = spyLogger();
    const reader = new TranscriptReader(
      "s",
      adapterWith(() => null),
      "/cwd",
      logger,
      fileSource,
      "/virtual-home",
    );
    const errors: string[] = [];
    reader.onError((e) => errors.push(e.kind));
    reader.onHook(postToolUse);
    await sleep(10);
    expect(errors).toHaveLength(0);
  });

  test("threads the resolved agent-home into the adapter transcript path", async () => {
    const seen: string[] = [];
    const adapter: AgentDefinition = {
      ...adapterWith(() => null),
      transcriptPath: (sessionId, cwd, agentHome) => {
        seen.push(agentHome);
        return `${agentHome}/projects/${cwd}/${sessionId}.jsonl`;
      },
    };
    const reader = new TranscriptReader(
      "s",
      adapter,
      "/cwd",
      spyLogger().logger,
      new FakeFileSource(0),
      "/substitute-home",
    );
    reader.onHook(postToolUse);
    await sleep(10);
    expect(seen).toContain("/substitute-home");
  });
});
