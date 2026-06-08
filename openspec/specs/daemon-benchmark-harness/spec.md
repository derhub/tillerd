# daemon-benchmark-harness Specification

## Purpose

TBD - created by archiving change rust-pty-daemon. Update Purpose after archive.

## Requirements

### Requirement: Comparative benchmark over the real socket protocol

The system SHALL provide a benchmark harness that drives a daemon implementation through its real control socket protocol — connecting, spawning sessions, streaming output, subscribing, and tearing down — rather than calling internal functions, so that measured cost reflects the full path including framing, fan-out, and snapshot production. The harness SHALL be able to run the same workloads against any conforming daemon binary by selecting the binary it launches.

#### Scenario: Same workload runs against either daemon

- **WHEN** the harness is pointed at the reference daemon binary and then at the alternative daemon binary
- **THEN** it SHALL execute the identical workload sequence against each over the real control socket and collect comparable measurements

#### Scenario: Measurement covers the full protocol path

- **WHEN** a workload streams output and produces snapshots
- **THEN** the harness SHALL measure cost as observed at the socket boundary, including framing and fan-out, not internal function timings

### Requirement: Defined comparative workloads

The harness SHALL include a fixed, reproducible set of workloads that exercise the daemon's hot paths: a rapid spawn workload creating many sessions, a sustained high-throughput output workload, a many-concurrent-sessions workload holding sessions open simultaneously, a subscribe-and-snapshot latency workload, and a reconnect-replay workload.

#### Scenario: Hot-path workloads are exercised

- **WHEN** the benchmark suite runs
- **THEN** it SHALL execute the spawn, sustained-throughput, concurrent-sessions, subscribe/snapshot, and reconnect-replay workloads against the selected daemon

#### Scenario: Workloads are reproducible

- **WHEN** the same workload is run twice against the same daemon binary on the same machine
- **THEN** the workload parameters (session counts, byte volumes, durations) SHALL be fixed inputs so the runs are comparable

### Requirement: Comparative report of resource and latency metrics

The harness SHALL emit a report comparing the daemon implementations on resident memory, byte-copy throughput, snapshot build time, and latency percentiles (at least median, 95th, and 99th percentile), so the difference between implementations is quantified rather than asserted.

#### Scenario: Report quantifies the difference

- **WHEN** the harness has run the workload set against two daemon binaries
- **THEN** it SHALL produce a single report presenting, per workload, the resident memory, throughput, snapshot build time, and latency percentiles for each daemon side by side

#### Scenario: Metrics are attributed per workload

- **WHEN** the report is produced
- **THEN** each metric SHALL be attributed to the workload that produced it and to the daemon binary under test
