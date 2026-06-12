## ADDED Requirements

### Requirement: Shell renders from the finalized token set

All shell styling SHALL come from the DESIGN.md token set, including the motion and
transition scale, the icon sizing token, and light-mode counterparts for every color
token. The terminal palette stays hardcoded and theme-independent as DESIGN.md
specifies. Token names are frozen at 0.0.6; later UI consumes them unchanged.

#### Scenario: Shell components use tokens only

- **WHEN** any shell component renders
- **THEN** its colors, spacing, motion, and icon sizes resolve from defined tokens, with
  no ad-hoc values outside the documented terminal palette exemption

#### Scenario: Light mode renders from token counterparts

- **WHEN** the light theme is active
- **THEN** every color token resolves to its light-mode counterpart with no missing
  definitions
