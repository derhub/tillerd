## 1. Reference host: inject the daemon-binary resolver

- [x] 1.1 Add an optional daemon-binary resolver argument to the reference host's adopt-or-spawn; default to the existing resolver so current callers are unaffected
- [x] 1.2 Export the resolver-options type from the reference host's public surface

## 2. Native host package

- [x] 2.1 Create the package (name, `type: module`, `exports`, `check-types`/`test` scripts) depending on the contracts/types package and the reference host package; add tsconfig and test tsconfig mirroring the reference host
- [x] 2.2 Implement the native daemon resolver: explicit override, then the native build-output location, then install locations; raise a typed not-found error naming the override variable and the build step (satisfies "Native daemon artifact resolved by build or discovery")
- [x] 2.3 Re-export the reference host's platform-port surface unchanged, overriding adopt-or-spawn to supply the native resolver so the native daemon is the default backend (satisfies "Host implements the engine platform ports backed by the native daemon" and "Native daemon is the default backend")

## 3. Tests

- [x] 3.1 Native resolver: resolves the build-output path, honors override precedence, and raises the typed not-found error naming the build step
- [x] 3.2 Reference host injection: adopt-or-spawn uses a supplied resolver and is unchanged when none is given
