# redaction Specification

## Purpose
Sensitive-data detection and redaction library: pattern/entropy/allowlist-based detection of credentials and structured PII (with checksum or context-keyword confirmation for low-confidence numeric classes), replacement of each detected span with a fixed [REDACTED] marker that preserves labeled keys, and a stdin/stdout CLI for external processes.
## Requirements
### Requirement: Sensitive-data detection

The redaction library SHALL detect sensitive substrings in an input string using three
layers: a catalog of named regular-expression patterns for well-known credential shapes and
structured PII, a Shannon-entropy heuristic for unknown high-randomness secrets gated by
minimum length and character class, and an allowlist that suppresses known structural values
(version-control object hashes, UUIDs, version-number-like sequences). Detection MUST be pure
and deterministic over its input. Pattern matches for low-confidence numeric classes (payment
card, phone) MUST require an additional signal — a checksum (Luhn) pass for payment cards, a
nearby context keyword for phone numbers — before being treated as a match.

#### Scenario: Known credential shape detected

- **WHEN** the input contains a value matching a credential pattern (such as a cloud access-key id or a JSON Web Token)
- **THEN** that value MUST be reported as a detected span

#### Scenario: Structured PII detected

- **WHEN** the input contains an email address, a checksum-valid payment card number, or a government identifier
- **THEN** that value MUST be reported as a detected span

#### Scenario: Unknown high-entropy secret detected

- **WHEN** the input contains a token above the configured length and entropy thresholds within the allowed character class and matched by no pattern
- **THEN** that token MUST be reported as a detected span

#### Scenario: Allowlisted structural value suppressed

- **WHEN** a candidate matches an allowlist entry (such as a version-control object hash or a UUID)
- **THEN** it MUST NOT be reported as a detected span

### Requirement: Redaction transform

The redaction library SHALL replace each detected span with the single fixed marker
`[REDACTED]`. The marker MUST NOT encode the detected class. Where the detected value is the
value of a labeled key/value pair — an environment assignment (`KEY=value`), a structured
field (`"key": "value"`), an authorization header, or a query parameter — only the value MUST
be replaced and the key, field name, or label MUST be preserved. A bare value with no
associated key MUST have its whole span replaced. When a detected secret cannot be cleanly
bounded, the library MUST fail closed and redact conservatively rather than leak. Input
containing no detected spans MUST be returned unchanged.

#### Scenario: Bare value replaced with marker

- **WHEN** a detected value appears with no associated key
- **THEN** the entire value MUST be replaced with `[REDACTED]`

#### Scenario: Labeled pair keeps its key

- **WHEN** a detected value is the value of a labeled pair such as `API_KEY=secret`
- **THEN** the result MUST be `API_KEY=[REDACTED]` with the key and separator unchanged

#### Scenario: Clean input unchanged

- **WHEN** the input contains no detectable sensitive data
- **THEN** the output MUST equal the input

### Requirement: Redaction command-line interface

The crate SHALL provide a command-line tool that reads text from standard input, applies
redaction, and writes the result to standard output, so the redactor is usable by any
external process.

#### Scenario: Stream redacted through the CLI

- **WHEN** text containing a secret is piped to the CLI on standard input
- **THEN** standard output MUST contain the same text with the secret replaced by `[REDACTED]`
- **AND** the process MUST exit zero

