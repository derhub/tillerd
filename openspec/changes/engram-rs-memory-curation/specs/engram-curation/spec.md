## ADDED Requirements

### Requirement: Daily memory curation

Once per day the memory layer SHALL run a single language-model call that reads the current global memory file and the day's digest and produces an updated global memory file. This SHALL be the only language-model use in the system.

#### Scenario: Daily job updates the global memory file

- **WHEN** the daily curation job runs
- **THEN** it MUST pass the current global memory file and the day's digest to the model
- **AND** it MUST write the model's output as the new global memory file

#### Scenario: Curation is the sole model use

- **WHEN** any other operation runs (capture, recall, consolidation, eviction)
- **THEN** it MUST complete without a language-model call

### Requirement: Bounded, deduplicated global memory

The global memory file SHALL be a compact snapshot bounded in size, covering user preferences, feedback, a project index, and cross-project relationships, with contradictions merged and stale entries dropped.

#### Scenario: Output stays within the size bound

- **WHEN** the curation job produces a new global memory file
- **THEN** the output MUST stay within the configured size bound

#### Scenario: Contradictions merged

- **WHEN** the day's digest contradicts an existing entry
- **THEN** the conflicting entries MUST be merged so only the current statement remains

### Requirement: Global memory injection

The global memory file SHALL be injected into every session at start, across all projects.

#### Scenario: Memory present at session start

- **WHEN** a session starts in any project
- **THEN** the current global memory file MUST be present in the injected context

### Requirement: Fact-graph population from curation

The daily curation job SHALL also extract structured facts from the digest and record them in the temporal fact graph, applying supersession so the graph stays consistent over time.

#### Scenario: Extracted facts recorded

- **WHEN** the curation job extracts a fact from the digest
- **THEN** that fact MUST be recorded in the temporal fact graph

#### Scenario: Extracted fact supersedes prior fact

- **WHEN** an extracted fact contradicts a currently-valid fact about the same subject and predicate
- **THEN** the prior fact MUST be superseded rather than duplicated

### Requirement: One-shot bootstrap

The memory layer SHALL provide a one-shot bootstrap that runs the curation job sequentially over historical digests, so memory can be populated from prior history on first adoption.

#### Scenario: Bootstrap processes historical digests in order

- **WHEN** bootstrap is invoked over a span of historical digests
- **THEN** the curation job MUST run once per historical digest in chronological order
- **AND** each run MUST update the global memory file and the fact graph
