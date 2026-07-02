## REMOVED Requirements

### Requirement: The per-entity TanStack surface is generated, not hand-written

**Reason**: The build-time hook emitter was abandoned (autogen-sdk change record); the shipped
entity-access surface is the generic runtime factories over the generated bindings, whose
requirement lives in `generated-entity-hooks` ("Entity access goes through generic runtime
factories over the generated bindings"). Typing, `meta.invalidates`, and optimistic semantics are
carried by that requirement; no per-entity generated hooks exist.
