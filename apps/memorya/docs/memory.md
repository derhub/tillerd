# Global memory (MEMORY.md)

> Deferred. The schema and pipeline below are designed but produced by the
> follow-up curation change; the foundation ships the fact-graph schema empty.

The primary curated output: a compact global memory injected into every session
in every project. Maintained by a once-daily LLM call (`memory_job`), not by
write-time interception.

```markdown
## User

- prefers bun over npm
- writes TypeScript, learning Rust

## Projects

- table-admin /path/to/table-admin admin UI
- table-api /path/to/dir-api REST API, related: table-admin

## Feedback

- no .js extensions on TS imports
- commit subject only, no body
```

## memory_job (one LLM call per day)

```
input:  current MEMORY.md + today's daily digest (all projects)
output: updated MEMORY.md (≤ ~800 tokens)
        + extracted facts -> temporal KG (with supersede)
```

Building `MEMORY.md` only in the daily job — which sees all projects' digests
rolled up — deduplicates naturally and keeps the file compact. The host agent's
own per-project memory files already carry intercepted facts; rendering them at
write time too would duplicate the same fact in that project's context.

`bootstrap` runs `memory_job` over historical digests in order, to populate
memory from prior history on first adoption.

This is the only LLM use in the system. Capture, recall, consolidation, and
eviction make no model call.
