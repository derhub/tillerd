# memorya-chunking

## Purpose

Defines how the memory layer splits documents into storable, embeddable chunks: heading-bounded sectioning at any level, size-bounded splitting that never breaks fenced code, dropping of trivial chunks, and heading-derived chunk titles.

## Requirements

### Requirement: Heading-bounded document chunking at any level

Document indexing SHALL split markdown at heading boundaries of any level, so a
chunk corresponds to a section rather than an arbitrary span. Content before the
first heading SHALL form its own chunk.

#### Scenario: Splits at every heading level

- **WHEN** a document contains headings of mixed levels
- **THEN** each heading MUST begin a new chunk, regardless of its level

#### Scenario: Preamble before the first heading is its own chunk

- **WHEN** a document has text before its first heading
- **THEN** that text MUST form a chunk of its own

### Requirement: Size-bounded sections split without breaking code

A section whose length exceeds a configured size bound SHALL be split into
smaller chunks at paragraph boundaries, carrying a small overlap between
adjacent chunks for continuity. A split MUST NOT occur inside a fenced code
block.

#### Scenario: Oversized section is split at paragraph boundaries

- **WHEN** a section exceeds the size bound
- **THEN** it MUST be split into multiple chunks at paragraph boundaries
- **AND** adjacent chunks MUST share a small overlap

#### Scenario: A fenced code block is never split

- **WHEN** an oversized section contains a fenced code block
- **THEN** no chunk boundary MUST fall inside that fence

### Requirement: Trivial chunks dropped

Chunks whose meaningful text falls below a minimum length SHALL be dropped, so
empty or near-empty sections are not stored or embedded.

#### Scenario: Below-minimum chunk is not stored

- **WHEN** a candidate chunk's meaningful text is below the minimum length
- **THEN** it MUST NOT be stored or embedded

### Requirement: Chunk titled by its heading

Each chunk SHALL be titled by the heading of the section it belongs to (falling
back to a short content prefix when there is no heading), so results are
distinguishable beyond the source file name.

#### Scenario: Section chunk carries its heading as title

- **WHEN** a chunk is produced from a section with a heading
- **THEN** its title MUST be that heading

#### Scenario: Headingless chunk falls back to a content prefix

- **WHEN** a chunk has no heading
- **THEN** its title MUST be a short prefix of its content
