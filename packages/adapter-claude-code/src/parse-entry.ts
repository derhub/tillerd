import type { ContentEvent } from "@athing/sdk";

interface TranscriptEntry {
  type?: string;
  role?: string;
  message?: {
    role?: string;
    content?: Array<{ type?: string; name?: string; input?: unknown }>;
    usage?: { input_tokens?: number; output_tokens?: number };
  };
  tool_use_id?: string;
  tool_name?: string;
  tool_input?: unknown;
  is_error?: boolean;
  session_id?: string;
  costUSD?: number;
  [key: string]: unknown;
}

export function parseTranscriptEntry(line: string, sessionId: string = ""): ContentEvent | null {
  let entry: TranscriptEntry;
  try {
    entry = JSON.parse(line) as TranscriptEntry;
  } catch {
    return null;
  }

  const sid = String(entry.session_id ?? sessionId);

  if (entry.type === "assistant" && entry.message?.content) {
    for (const block of entry.message.content) {
      if (block.type === "tool_use" && block.name) {
        return {
          kind: "tool_use",
          sessionId: sid,
          toolName: block.name,
          toolInput: block.input ?? {},
        };
      }
    }
  }

  if (entry.type === "tool_result" || (entry.type === "assistant" && entry.tool_name)) {
    const name = entry.tool_name ?? "";
    if (name === "str_replace_editor" || name === "write_file" || name === "create_file") {
      const input = entry.tool_input as Record<string, unknown> | undefined;
      return {
        kind: "edit",
        sessionId: sid,
        filePath: String(input?.["path"] ?? input?.["file_path"] ?? ""),
        oldContent: typeof input?.["old_string"] === "string" ? input["old_string"] : undefined,
        newContent: typeof input?.["new_string"] === "string" ? input["new_string"] : undefined,
      };
    }
  }

  if (entry.type === "usage" || entry.message?.usage) {
    const usage = entry.message?.usage;
    if (usage) {
      return {
        kind: "usage",
        sessionId: sid,
        inputTokens: usage.input_tokens ?? 0,
        outputTokens: usage.output_tokens ?? 0,
        costUsd: typeof entry.costUSD === "number" ? entry.costUSD : undefined,
      };
    }
  }

  return null;
}
