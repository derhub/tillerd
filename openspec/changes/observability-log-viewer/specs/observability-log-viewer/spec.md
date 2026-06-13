## ADDED Requirements

### Requirement: Global log-viewer route

The application SHALL present a log viewer at its own global route, scoped to the app shell.
The log view SHALL NOT be a session surface and SHALL NOT occupy a placement in any session's
panel tree.

#### Scenario: Reaching the log viewer

- **WHEN** the user navigates to the log-viewer route
- **THEN** the application shows the log view in the content area, independent of any session

#### Scenario: Not a session surface

- **WHEN** a session is active and the user opens the log-viewer route
- **THEN** the view is shown as app-shell chrome and consumes no session placement or surface

### Requirement: Host-agnostic log source port

The viewer SHALL read logs through a source abstraction exposing `list` (available log files
with sizes), `size(file)` (current byte length, or absent when the file does not exist), and
`read(file, offset, length)` (a raw byte range, short at end of file). The viewer's tail,
merge, parse, and filter logic SHALL depend only on this abstraction and SHALL NOT reference a
host-specific transport, so an alternate host (a web/server host) can satisfy the same
contract without changing the viewer.

#### Scenario: Listing available log files

- **WHEN** the viewer requests the available log files
- **THEN** the source returns each file's name and current byte size

#### Scenario: Reading a byte range short at end of file

- **WHEN** the viewer reads `length` bytes from `offset` and fewer bytes remain
- **THEN** the source returns only the available bytes

#### Scenario: Desktop adapter satisfies the port

- **WHEN** the viewer runs on the desktop host
- **THEN** the port is satisfied by a host listing of the runtime logs directory plus the
  host's existing file size and read operations

### Requirement: Logs from all services merged in time order

The viewer SHALL present records drawn from every service's log file as a single stream
ordered by record timestamp, regardless of which file each record came from.

#### Scenario: Interleaved by timestamp

- **WHEN** the log files for two services contain records with interleaved timestamps
- **THEN** the viewer presents the records in one list ordered by timestamp

### Requirement: Near-live tail

The viewer SHALL surface newly appended records without a manual reload, by polling each
file's size and reading the appended bytes. A record split across reads (a partial trailing
line) SHALL NOT be shown until its terminating newline arrives.

#### Scenario: New record appears

- **WHEN** a new JSON line is appended to a tailed log file
- **THEN** the viewer shows the new record without a reload

#### Scenario: Partial line is not shown until complete

- **WHEN** a read ends mid-line, splitting one record across two reads
- **THEN** the viewer buffers the partial bytes and shows the record only once its newline is
  read

### Requirement: History backfill and load-older

On open the viewer SHALL show recent records from the tail of each file, and SHALL load older
records on demand by reading earlier byte ranges.

#### Scenario: Recent history on open

- **WHEN** the viewer opens
- **THEN** it shows the most recent records from each available log file

#### Scenario: Loading older records

- **WHEN** the user requests older records
- **THEN** the viewer reads an earlier range of the file and prepends those records

### Requirement: Filter and search records

The viewer SHALL filter the shown records by minimum severity level, by free-text query over
the body and attributes, and by `component` and `session.id` facets.

#### Scenario: Filter by level

- **WHEN** the user selects a minimum severity level
- **THEN** records below that level are hidden

#### Scenario: Free-text search

- **WHEN** the user enters a query
- **THEN** only records whose body or attributes contain the query are shown

#### Scenario: Facet by component or session

- **WHEN** the user selects a `component` or `session.id` value
- **THEN** only records carrying that value are shown

### Requirement: Records render their OpenTelemetry fields

Each record SHALL display its timestamp, severity, body, attributes, and resource without
reshaping the field semantics defined by `observability-logging`.

#### Scenario: Record fields shown

- **WHEN** a record is displayed
- **THEN** its timestamp, severity, body, originating `service.name`, and any `session.id` are
  visible
