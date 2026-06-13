# 0004. Hybrid adapters: declarative config plus parse functions

- Status: accepted
- Date: 2026-06-01

## Context

An `AgentDefinition` must carry everything agent-specific. Pure-data adapters are clean for stable structural config but turn parsing (hook payloads, transcript entries) and encoding (the transcript path's cwd rule) into a config-DSL plus a growing engine interpreter — indirection, hard debugging, an expressiveness ceiling. Pure-code adapters are maximally flexible but lose the clarity of declarative config. (An earlier rationale for pure data — cross-language engine swap — no longer applies, since there is a single engine.)

## Decision

An `AgentDefinition` is a hybrid:

- declarative DATA for stable config: launch template (command, args with placeholders, flags), hook-install spec (settings path, command template, event list), supported CLI version range;
- small FUNCTIONS where logic varies per agent: `parseHook(raw) -> HookEvent`, `transcriptPath(sessionId, cwd) -> path`, `parseTranscriptEntry(line) -> content`.

The engine consumes the data and calls the functions; it never hard-codes the agent's payload, transcript, or path shapes.

## Consequences

- Each concern lives where it is cheapest to maintain; parse functions are unit-testable and debuggable.
- Because `parseHook` normalizes to the fixed contract enum, the engine maps enum->status generically (no per-adapter status table).
- Adapters are code (a small module + tests), not pure data; they depend only on the sdk.
