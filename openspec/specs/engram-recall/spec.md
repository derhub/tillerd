# engram-recall

## Purpose

Defines how the memory layer retrieves stored content: hybrid lexical-and-vector retrieval with temporal rerank, progressive disclosure of results, staleness-checked session-start injection, archive fallback, and the agent-facing and human-facing recall surfaces.

## Requirements

### Requirement: Hybrid retrieval

Recall SHALL combine semantic vector search and lexical full-text search over non-archived content, fuse the two result lists, and apply a temporal rerank, returning a ranked result set. Recall MUST NOT invoke an external language model.

#### Scenario: Vector and lexical results fused

- **WHEN** a recall query is issued
- **THEN** the query MUST be embedded and matched by vector similarity
- **AND** the query MUST be matched lexically by full-text search
- **AND** the two result lists MUST be fused into one ranked list

#### Scenario: Adaptive weighting by query shape

- **WHEN** the query is symbol-like rather than prose
- **THEN** lexical matching MUST be weighted more heavily than semantic matching

#### Scenario: Temporal rerank applied

- **WHEN** results are ranked
- **THEN** more recent items MUST be boosted
- **AND** facts whose validity has ended MUST be penalized

#### Scenario: Access statistics updated on hit

- **WHEN** a result is returned from recall
- **THEN** its last-accessed time and access count MUST be updated

### Requirement: Progressive disclosure

The agent-facing recall surface SHALL disclose results progressively: a compact session-start layer, an on-demand search layer returning identifiers with titles and snippets, and a full-content expansion layer.

#### Scenario: Session-start context is compact

- **WHEN** a session starts
- **THEN** the injected context MUST contain recent digests subject to a staleness check and the list of known projects
- **AND** it MUST NOT inline full chunk contents

#### Scenario: Search returns identifiers and snippets

- **WHEN** the agent issues a recall query
- **THEN** each result MUST be returned as an identifier with a title and a snippet

#### Scenario: Expansion returns full content

- **WHEN** the agent expands a result identifier
- **THEN** the full content of that item MUST be returned

### Requirement: Recent-digest staleness check

Session-start injection SHALL include a recent digest only when no newer captured content exists after the digest, so stale summaries are not presented as current state.

#### Scenario: Fresh digest shown

- **WHEN** a digest's timestamp is later than the most recent captured chunk it covers
- **THEN** the digest MUST be eligible for injection

#### Scenario: Stale digest withheld

- **WHEN** captured content exists that is newer than the digest
- **THEN** the digest MUST be withheld from injection

### Requirement: Archive fallback

When recall finds no sufficiently strong match, it SHALL report uncertainty and offer to search the archive, and on confirmation SHALL search archive shards newest-first with the option to search progressively older shards.

#### Scenario: Weak match reports uncertainty

- **WHEN** the best result score is below the confidence threshold
- **THEN** recall MUST return an uncertain result that offers an archive search

#### Scenario: Archive searched newest-first on confirmation

- **WHEN** the user confirms an archive search
- **THEN** the newest archive shard MUST be searched first
- **AND** the user MUST be able to continue searching progressively older shards

### Requirement: Agent-facing and human-facing surfaces

Recall SHALL be exposed to the agent over a tool interface and to a human over a viewer bound to the loopback interface only.

#### Scenario: Viewer is loopback-only

- **WHEN** the human viewer is served
- **THEN** it MUST bind to the loopback interface only
