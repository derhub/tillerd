## 1. Read-through cache + revalidation

- [x] 1.1 Add an mtime-keyed parsed-file cache to `FsBackend` and route entity-file reads through it: stat → reuse on equal mtime, re-read + replace on change (TDD: repeated-read identity, external-change re-read, unchanged-served scenarios).
- [x] 1.2 Make mutating methods write through the cache and id→path index under the existing write lock so reads reflect own writes (TDD: read-after-write scenario).

## 2. Lazy index

- [x] 2.1 Defer `build_index` at `open`; build the id→path index on demand (live + `.archive/`), keeping `seed_defaults` eager (TDD: get-by-id-after-open and empty-tree-seed scenarios).

## 3. Verify gate

- [x] 3.1 Fix-all: `cargo fmt --all`, `cargo clippy --all-targets --locked --workspace -- -D warnings`, `cargo test --workspace` all green; every spec scenario has a passing unit test.
