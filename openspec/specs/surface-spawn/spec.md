# surface-spawn Specification

## Purpose

Surface spawn is the single path through which any surface kind starts a process: it hands a fully-resolved command to the pseudo-terminal service and returns the per-surface proxy that the surface runtime owns for I/O and status.

## Requirements

### Requirement: A surface is spawned by handing a resolved command to the pseudo-terminal service

The surface runtime SHALL spawn a surface's process by passing a resolved command — executable,
arguments, environment, and working directory — to the pseudo-terminal service, keyed by the
surface identifier. The spawn path SHALL be identical regardless of surface kind. An absent command
SHALL spawn the login shell.

#### Scenario: A command is spawned

- **WHEN** a surface is launched with an executable and arguments
- **THEN** the pseudo-terminal service spawns that executable with those arguments, under the given working directory and environment, keyed by the surface identifier

#### Scenario: Absent command spawns the login shell

- **WHEN** a surface is launched with no command
- **THEN** the pseudo-terminal service spawns the login shell

### Requirement: The spawn yields the per-surface proxy

A successful spawn SHALL produce the per-surface proxy the surface runtime owns: raw output bytes
and status flow over the event sink tagged with the surface identifier, and input is accepted and
flushed in order.

#### Scenario: Output streams from the spawned process

- **WHEN** a spawned process emits output
- **THEN** the exact bytes are delivered over the event sink, tagged with the surface identifier
