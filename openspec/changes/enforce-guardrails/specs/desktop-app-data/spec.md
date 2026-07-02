## REMOVED Requirements

### Requirement: Native store for user preferences

**Reason**: Zero consumers. The renderer's preference reads/writes (`pref_get`/`pref_set`) belonged
to the retired TS-engine path; settings live in the orchestrator settings store.

### Requirement: Native session registry

**Reason**: Zero consumers. The orchestrator store (`tillerd.db`) owns session data; the
JSON-file registry (`registry_*` commands, `StoreState`) was its pre-orchestrator stand-in.
