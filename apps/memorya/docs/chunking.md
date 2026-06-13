# Chunking

How project markdown becomes searchable `doc` chunks. Source: `src/indexer.rs`
(`chunk_markdown` + `split_large`).

````
markdown file
     │
     ▼
 PASS 1 — section split (chunk_markdown)
   walk lines, track fenced-code state (``` / ~~~)
     heading (^#{1,6} ) AND not in fence  -> start a new section
     else                                 -> append to current section
   keep a section only if body_chars ≥ MIN_BODY_CHARS
     (chars in non-heading, non-blank lines -> drops bare headings / empty sections)
     │  Vec<Section{ title: heading?, content }>
     ▼
 PASS 2 — size bound
   content.chars() ≤ MAX_CHUNK_CHARS  -> keep as one chunk
   content.chars() >  MAX_CHUNK_CHARS  -> split_large(...)
     │
     ▼
 split_large — paragraph split, fence-safe
   accumulate lines, track fence state
   cut when:  blank line AND not in fence AND buf ≥ MAX_CHUNK_CHARS AND more lines left
   on cut:    emit chunk, carry last OVERLAP_LINES forward,
              but FILTER OUT fence markers (a chunk can never inherit a half-open ```)
     │  Vec<DocChunk{ title, content }>
     ▼
   title = heading  (fallback: file name)
   stored as kind='doc' chunk -> FTS5 + embedding (async)
````

## Parameters

| Constant          | Value | Why                                                                 |
| ----------------- | ----- | ------------------------------------------------------------------- |
| `MAX_CHUNK_CHARS` | 1500  | cap so one large section is not averaged into one diluted embedding |
| `OVERLAP_LINES`   | 2     | continuity across a split                                           |
| `MIN_BODY_CHARS`  | 3     | drop bare-heading / empty chunks                                    |

## Rules

1. **Split at any heading level** — `#` through `######`, not just `##`/`###`. Content before the first heading is its own untitled chunk.
2. **Oversized sections split at paragraph boundaries** with carried overlap; a boundary is never placed inside a fenced code block.
3. **Trivial chunks dropped** — a section whose body (excluding headings and blank lines) is below `MIN_BODY_CHARS` is not stored or embedded.
4. **Titled by heading** — falls back to the source file name when there is no heading.

## Fence guards (the non-obvious bits)

- A `#`-prefixed line **inside** a ``` block is **not** treated as a heading, so fenced pseudo-headings don't start new chunks.
- When carrying overlap forward after a split, fence markers are stripped, so no chunk ends up with a single unbalanced ```.

These matter because docs are full of code-block diagrams; naive blank-line splitting would cut through a fence and mangle retrieval.

## Indexing

On `SessionStart` the indexer scans `{cwd}/**/*.md`, honoring `.gitignore`,
`.ignore`, and the global gitignore (and pruning `node_modules/`, `target/`,
`.git/`, `dist/`, `build/`). Prior `doc` chunks for the project are deleted, then
fresh chunks are stored — documents are indexed **verbatim**. Runs async; never
blocks the session.

Behaviour is pinned by tests in `src/indexer.rs`: all-level split, preamble
chunk, fence-not-heading, oversize split, fence-never-split, min-drop, heading
title, and gitignore honoring.
