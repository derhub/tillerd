## 1. Chunking (engram-chunking)

- [x] 1.1 Split markdown at every heading level (`#`–`######`); content before the first heading is its own chunk
- [x] 1.2 Split sections over a size bound at paragraph boundaries with carried overlap; never place a boundary inside a fenced code block
- [x] 1.3 Drop chunks whose meaningful text is below a minimum length
- [x] 1.4 Title each chunk by its heading, falling back to a short content prefix
- [x] 1.5 Tests: all-level split, preamble chunk, oversize split + overlap, fence never split, min-length drop, heading title

## 2. Embeddings (engram-embeddings)

- [x] 2.1 Add the static embedding model as a normal dependency (default features include the model hub) — it is the only production embedder
- [x] 2.2 Implement the static-model embedder behind the `Embedder` trait: load by hub repo id or local path, record model id + dim
- [x] 2.3 Download the model from the hub on first use and cache it; reuse the cache offline afterward
- [x] 2.4 `Engram::open` uses the default static model; tests use a deterministic in-crate stub at the embedder boundary
- [x] 2.5 Safe model switching: drop stored embeddings whose model differs from the active one on startup; rely on out-of-band backfill (search already filters by active model)
- [x] 2.6 Tests: stub-backed pipeline works; stale-model vectors dropped on open and ignored by recall; missing embeddings backfilled; real model verified (ignored, downloads)
