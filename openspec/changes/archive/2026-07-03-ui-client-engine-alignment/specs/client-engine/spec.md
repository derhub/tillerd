## MODIFIED Requirements

### Requirement: Server-state cache is the single sync axis

All server-state reads SHALL resolve through the Query cache (`useQuery`/`useSuspenseQuery` over
the generated `query()` factories); components SHALL NOT fetch in effects and mirror results into
local state. Mutations SHALL refresh by invalidation. One exception is permitted: a
**high-frequency stream** (terminal output, live log tail) MAY feed the render through a local
bounded buffer, because patching the Query cache per record would re-render the world on every
frame; the durable part of such a feature (backlog, file lists) still resolves through the cache
and revalidates by invalidation.

#### Scenario: A read resolves through the query cache

- **WHEN** a component needs server state
- **THEN** it reads via a Query hook over the `query()` factory and renders from the cache

#### Scenario: A mutation refreshes by invalidation, not imperative refresh

- **WHEN** a mutation succeeds
- **THEN** affected queries refresh via declared invalidation keys, never a hand-called refresh

#### Scenario: A high-frequency stream renders from a bounded local buffer

- **WHEN** a component renders a high-frequency stream (PTY bytes, live log records)
- **THEN** records append to a bounded local buffer merged at render time
- **AND** the feature's durable reads (backlog, lists) still resolve through the Query cache and
  revalidate via invalidation
