See `README.md` for the project overview, Bun-first toolchain, and dev commands; 
`docs/adr/` for architectural decisions.
`CONTEXT.md` for language and terminology; `ROADMAP.md` for milestones and timelines;
`DESIGN.md` for design principles and tokens; 

## Rust dev loop

- Tests: `cargo nextest run` (2-3x faster than `cargo test`; excludes doctests — run `cargo test --doc` separately if any exist).
- Fast feedback: prefer `cargo check` over `cargo build` when you only need type errors.
- Never wire sccache into interactive builds — it disables incremental compilation (net loss for the edit-test loop). Cold rebuilds/CI only.
