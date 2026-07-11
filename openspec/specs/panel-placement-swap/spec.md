# panel-placement-swap Specification

## Purpose
TBD - created by archiving change ux-ui-overhaul. Update Purpose after archive.
## Requirements
### Requirement: Drag a panel leaf onto another to swap placements

Dragging a panel leaf's header and dropping it onto another leaf in the same session SHALL swap the two surfaces' placements. The swap SHALL persist (both surfaces resume in their new slots after reload) and the panel geometry SHALL be unchanged — only the surface-to-slot binding swaps.

#### Scenario: Swap two terminals

- **WHEN** the user drags terminal A's panel header onto terminal B's panel
- **THEN** A renders in B's slot and B in A's slot, both PTYs uninterrupted

#### Scenario: Swap survives reload

- **WHEN** the user swaps two panels and reloads the window
- **THEN** the surfaces resume in their swapped slots

### Requirement: Drop targets are indicated during drag

While dragging a panel leaf, valid drop targets SHALL show a visible highlight under the
pointer; dropping outside any valid target SHALL cancel with no change.

#### Scenario: Cancelled drag changes nothing

- **WHEN** the user drags a panel header and releases it over the sidebar
- **THEN** no swap occurs and the layout is unchanged

