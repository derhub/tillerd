## REMOVED Requirements

### Requirement: Core packages remain logging-library-agnostic

**Reason**: The TS packages it governed are retired: `@tillerd/engine` (removed earlier),
`@tillerd/sdk`, and `@tillerd/logger` (both deleted in this change, zero consumers). Process
logging is owned by Rust `tracing` (`tillerd_paths::logging`); there is no TS core package to keep
logging-library-agnostic.
