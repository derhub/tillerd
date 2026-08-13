## ADDED Requirements

### Requirement: Versioned layout clean cutover

The UI SHALL serialize panel geometry as `{ "version": 1, "root": <panel-tree> }` and SHALL accept only that versioned envelope with valid split-group sizes. An unversioned, unsupported-version, or invalid non-null blob SHALL produce an incompatible-layout error. The UI SHALL NOT infer sizes, migrate the blob, overwrite it, or render a replacement tree. A null layout SHALL continue to render the single-empty-leaf default.

#### Scenario: Versioned nested geometry restores

- **WHEN** a session stores a version-1 layout containing nested split groups with valid sizes
- **THEN** the UI restores the complete tree and each group's independent sizes

#### Scenario: Unversioned layout is rejected

- **WHEN** a session stores the previous unversioned panel-tree blob
- **THEN** the UI renders a blocking incompatible-layout alert without inferring, replacing, or overwriting geometry, while the existing sidebar remains available to discard the development session

## MODIFIED Requirements

### Requirement: Layout updated on panel mutation

The UI SHALL send a store-layout request to the orchestrator after every panel mutation that changes persisted state, including split, close, mode change, content assignment, completed divider resize, and divider reset. The in-memory panel tree and the persisted layout SHALL remain consistent: a page reload SHALL produce the same panel tree and child sizes as were present before the reload.

#### Scenario: Layout written after split

- **WHEN** the user splits a panel
- **THEN** the UI sends a store-layout request with the updated tree before the next render cycle completes

#### Scenario: Layout written after resize

- **WHEN** the user completes a divider drag or resets a divider
- **THEN** the UI sends a store-layout request containing the updated normalized sizes

#### Scenario: Reload restores panel tree

- **WHEN** the user reloads the page while a session with a persisted layout is active
- **THEN** the panel tree and every split group's child sizes are initialized from the persisted versioned layout
