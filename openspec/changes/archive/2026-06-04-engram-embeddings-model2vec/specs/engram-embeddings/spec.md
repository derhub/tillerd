## ADDED Requirements

### Requirement: Semantic embeddings from a static, CPU-only model

The memory layer SHALL embed chunks and queries with a static, CPU-only
embedding model that produces semantically meaningful vectors, behind a stable
embedder interface. Each embedding row MUST record the producing model's
identity and dimensionality.

#### Scenario: Embedding produced on CPU and tagged with its model

- **WHEN** a chunk or query is embedded with the static model configured
- **THEN** the vector MUST be produced on CPU without a GPU
- **AND** the stored embedding MUST record which model and dimensionality produced it

#### Scenario: Semantically related content ranks above unrelated content

- **WHEN** a query and a stored passage are related in meaning but share few words
- **THEN** that passage MUST rank above an unrelated passage of similar length

### Requirement: First-run model download, then offline reuse

On first use the embedding model SHALL be fetched from a remote model hub and
cached locally; subsequent runs SHALL load it from the local cache without any
network access.

#### Scenario: Model downloaded on first use

- **WHEN** the embedder initializes and the model is not yet cached
- **THEN** it MUST download the model from the hub and cache it locally

#### Scenario: Cached model reused without network

- **WHEN** the embedder initializes and the model is already cached
- **THEN** it MUST load from the local cache with no network call

### Requirement: Safe model switching

Recall SHALL only compare query vectors against stored vectors produced by the
same model, and content lacking an embedding for the active model MUST be
re-embedded out-of-band.

#### Scenario: Recall ignores vectors from a different model

- **WHEN** the configured model differs from the one that produced some stored vectors
- **THEN** recall MUST compare the query only against stored vectors from the active model

#### Scenario: Missing embeddings are backfilled

- **WHEN** content has no embedding for the active model
- **THEN** it MUST be re-embedded out-of-band for that model
