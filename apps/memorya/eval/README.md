# Retrieval evaluation

Deterministic IR metrics (Recall@K, MRR, NDCG@10) over a committed dataset. No
language model in scoring; the default metric is Recall@5.

## Run

```
cargo run --bin memorya-eval        # uses the real embedding model (downloads on first run)
```

Reports overall and per-category metrics plus mean latency and result size, at a
fixed retrieval depth. Run twice with different configurations (e.g. a different
embedding model) to compare at equal depth on the same dataset.

## Dataset

- `corpus.jsonl` — `{ id, title, content }` chunks (the searchable corpus).
- `queries.jsonl` — `{ query, gold, category }` where `gold` is the corpus ids
  that correctly answer the query and `category` is `simple` / `complex` /
  `multi-hop`.

## Regenerating the dataset (manual, one-time)

The dataset is a golden fixture — regenerated deliberately, not at run time:

1. **Collect** — chunk real project docs with the same heading-boundary chunker
   used in production.
2. **Clean** — drop chunks below the minimum body length; strip secrets / paths.
3. **Annotate** — author queries per chunk (a one-time LLM pass works): one
   straightforward `simple` question, one reasoning-required `complex` question,
   and `multi-hop` questions that need 2+ related chunks. Label each with its
   gold corpus ids.

Commit the result and review the diff when it changes.
