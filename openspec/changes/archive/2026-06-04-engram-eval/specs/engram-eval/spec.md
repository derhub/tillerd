## ADDED Requirements

### Requirement: Labeled retrieval dataset

The evaluation SHALL run against a committed dataset of labeled queries, each
naming the chunk(s) that correctly answer it and a category (e.g. simple,
complex, multi-hop). The dataset SHALL be a stored fixture, regenerated
deliberately rather than at run time.

#### Scenario: Each query carries its gold chunks and category

- **WHEN** the dataset is loaded
- **THEN** every query MUST have one or more gold chunk identifiers and a category

#### Scenario: Dataset is a committed fixture

- **WHEN** the evaluation runs
- **THEN** it MUST read the dataset from a stored fixture, not generate it on the fly

### Requirement: Deterministic retrieval metrics

The evaluation SHALL score recall with deterministic information-retrieval
metrics — Recall@K (for K of 1, 5, 10), Mean Reciprocal Rank, and NDCG@10 —
computed without any language-model call. Recall@5 SHALL be the primary metric.

#### Scenario: Metrics computed from ranked results

- **WHEN** a query's ranked results are scored against its gold chunks
- **THEN** Recall@1, Recall@5, Recall@10, MRR, and NDCG@10 MUST be computed deterministically

#### Scenario: No model call during scoring

- **WHEN** the evaluation scores results
- **THEN** it MUST complete without invoking a language model

### Requirement: Per-category and cost reporting

The evaluation SHALL report metrics broken down by query category and, alongside
accuracy, the mean retrieval latency and mean result size, so accuracy is never
read in isolation from cost.

#### Scenario: Report includes category breakdown and cost

- **WHEN** the evaluation finishes
- **THEN** the report MUST include metrics per category
- **AND** the report MUST include mean retrieval latency and mean result size

### Requirement: Reproducible comparison across configurations

The evaluation SHALL be runnable as a command over the same dataset so two
configurations (e.g. different embedding models or fusion settings) can be
compared at equal retrieval depth.

#### Scenario: Same dataset compares two configurations

- **WHEN** the evaluation is run twice with different retrieval configurations against the same dataset and retrieval depth
- **THEN** the two reports MUST be directly comparable
