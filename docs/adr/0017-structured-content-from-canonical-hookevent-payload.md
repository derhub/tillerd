# 0017. Structured content from the canonical HookEvent payload

- Status: accepted
- Date: 2026-06-07
- Supersedes: ADR-0006

## Context

ADR-0006 recovered structured content (tool calls, edits, usage/cost) by reading the on-disk session
transcript, triggered by the hook plane, via the adapter's `parseTranscriptEntry`. That coupled
content to a transcript file format and to a second parse path, and it required a TypeScript parser
(`parseTranscriptEntry`/`transcriptPath`) alongside the Rust hook parser — two parsers to keep in
sync.

With the gate as the universal hook ingress (ADR-0016), the gate already normalizes raw hook input to
a canonical `HookEvent` once, via an injected adapter. That canonical payload can carry the same
structured content the transcript provided (prompt content; tool name/input/response; turn index).

## Decision

Supersede ADR-0006. Structured content comes from the canonical `HookEvent` **payload**, not from
reading the transcript. `parseTranscriptEntry` and `transcriptPath` are removed, along with the
engine's transcript reader. The engine maps `HookEvent.type` to status and `HookEvent.payload` to
content.

The canonical `HookEvent` has a typed per-type payload rich enough for both status and
content/capture; the gate's injected adapter fills it, and parser unit tests assert the mapping. The
agent adapter is therefore **single-language (Rust `parse_hook` only)** — there is no cross-language
parser to keep in sync. ADR-0004 stays satisfied (one adapter, one language); ADR-0005 is honored
(the gate, as the ingress, calls the adapter's parse function).

## Consequences

- One parse path (Rust `parse_hook`), one canonical content model; no transcript file coupling and no
  byte-offset/rewrite-detection machinery.
- Usage/cost and any detail only the transcript carried are dropped with the feature; the UI still
  shows live activity via the raw PTY stream. Richer content later is a new, separate source — not a
  revival of transcript coupling.
- Memory capture and the engine consume the same canonical `HookEvent`; capture maps it to chunks
  with no raw-format parsing.
- The `AgentDefinition`/adapter contract loses `parseTranscriptEntry` and `transcriptPath`; the
  TypeScript agent adapter no longer carries parse functions.
