# Recall

Hybrid retrieval over chunks: brute-force vector cosine fused with FTS5 lexical
search, reranked by recency. No model call beyond embedding the query. Source:
`src/search.rs` + `Engram::recall` / `rank` / `search`.

## Progressive disclosure

```
Layer 1 — SessionStart (~200-400 tokens)
  → MEMORY.md (when present)
  → recent digests, staleness check: only if ts_digest ≥ ts_last_chunk
  → project list from sessions

Layer 2 — on-demand recall
  recall(query) → Found({id, title, snippet}) | Uncertain { offer_archive }
  search(query) → one-shot: ranked results with full content inline

Layer 3 — expand / archive
  expand(id)            → full chunk content
  archive_recall(query) → opted-in, newest shard first, progressive deepening
```

## recall() internals

```
recall(query, now)
  → embed query                         (model2vec, μs)
  → vector_search: brute-force cosine over non-archived chunk vectors of the active model
  → lexical_search: FTS5 BM25 (porter unicode61), terms OR-joined and quoted
  → adaptive weighting:
       symbol-like query → lexical weighted higher (0.7 / 1.5)
       prose query       → balanced (1.0 / 1.0)
  → RRF fuse (k=60)
  → recency rerank: score *= 1 + 1/(1 + days_since_ts)
  → confidence gate: empty, or best cosine < 0.30 with no lexical hit → Uncertain
  → touch hits (last_accessed, access_count++)
  → return {id, title, snippet}
```

`rank(query, k)` returns the ranked ids without the confidence gate — used by the
[evaluation harness](../eval/README.md).

## Lexical query handling

FTS5 terms are quoted (to avoid syntax errors on arbitrary input) and **OR-joined**
so recall is forgiving — any matching term contributes rather than requiring all
of them.

## Why brute force

At engram's scale (tens of thousands of active vectors) a full cosine scan is
sub-millisecond, so no ANN index is needed. The consolidation + eviction policy
keeps the active vector set bounded, so this holds indefinitely. Measured by the
[eval harness](../eval/README.md): ~1ms per query.
