## ADDED Requirements

### Requirement: Shell renders from the finalized token set

The shell's own components SHALL take their styling from the DESIGN.md token set, including
the motion and transition scale, the icon sizing token, and light-mode counterparts for every
color token. Three classes of value are exempt: the terminal palette (hardcoded and
theme-independent as DESIGN.md specifies), vendored component primitives (e.g. shadcn `ui/`
internals such as focus-ring widths and geometry), and content-relative units (e.g. `ch`
measures). Token names are frozen at 0.0.6; later UI consumes them unchanged.

#### Scenario: Shell components use tokens only

- **WHEN** a shell component renders
- **THEN** its colors, spacing, motion, and icon sizes resolve from defined tokens, with no
  ad-hoc values outside the documented exemptions (terminal palette, vendored primitives,
  content-relative units)

#### Scenario: Light mode renders from token counterparts

- **WHEN** the light theme is active
- **THEN** every color token resolves to its light-mode counterpart with no missing
  definitions
