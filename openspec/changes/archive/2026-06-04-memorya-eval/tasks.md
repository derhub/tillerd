## 1. Dataset

- [x] 1.1 Define the dataset record: `{ query, gold_chunk_ids, category }` with categories simple / complex / multi-hop
- [x] 1.2 Add committed golden fixtures under `apps/memorya-rs/eval/`: `corpus.jsonl` (searchable chunks) and `queries.jsonl` (`{ query, gold, category }`)
- [x] 1.3 Document the one-time generation recipe (chunk real docs → LLM-author queries → label gold chunks); generation is manual, not part of the run

## 2. Metrics

- [x] 2.1 Implement Recall@K (K = 1, 5, 10) over a ranked result list and gold set
- [x] 2.2 Implement Mean Reciprocal Rank
- [x] 2.3 Implement NDCG@10
- [x] 2.4 Tests: known rankings yield known metric values (including the empty / no-hit case)

## 3. Harness

- [x] 3.1 Load the dataset fixture and, for each query, index the referenced docs into a scratch store and run `recall`
- [x] 3.2 Map each query's ranked chunk ids to ranks of its gold chunks; record per-query latency and result size
- [x] 3.3 Aggregate metrics overall and per category; compute mean latency and mean result size

## 4. Command + report

- [x] 4.1 Add an `eval` path (CLI subcommand or `eval/` binary) that runs the harness over the dataset at a fixed retrieval depth
- [x] 4.2 Print a report: Recall@1/5/10, MRR, NDCG@10 overall and per category, plus mean latency and mean result size; mark Recall@5 as primary
- [x] 4.3 Make the run reproducible so two configurations can be compared at equal depth on the same dataset
