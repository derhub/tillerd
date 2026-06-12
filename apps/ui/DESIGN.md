---
version: alpha
name: tillerd
description: A compact, zero-radius developer-tool shell built on the VSCode
  2026 palette. Two canonical modes — dark (#191a1b near-black with a steel-blue
  accent) and light (#fafafd off-white with a deep-blue accent) — share identical
  semantic roles. The terminal surface is always dark regardless of host mode.
  All depth is expressed through 1px borders, never shadows. Density is
  maximized: 12px root, tight spacing, every panel pixel earns its place.

colors:
  # Dark mode (primary)
  background: "#191a1b"
  foreground: "#bfbfbf"
  card: "#202122"
  card-foreground: "#bfbfbf"
  popover: "#202122"
  primary: "#297aa0"
  primary-foreground: "#ffffff"
  secondary: "#242526"
  secondary-foreground: "#bfbfbf"
  muted: "#2c2d2e"
  muted-foreground: "#8c8c8c"
  accent: "#2c2d2e"
  accent-foreground: "#ededed"
  destructive: "#f48771"
  border: "#2a2b2c"
  input: "#333536"
  ring: "#3994bc"
  # Light mode (role-identical counterparts)
  background-light: "#fafafd"
  foreground-light: "#202020"
  primary-light: "#0069cc"
  muted-foreground-light: "#606060"
  border-light: "#e4e5e6"
  destructive-light: "#ad0707"
  # Terminal (hardcoded dark, theme-independent)
  terminal-bg: "#0d1117"
  terminal-fg: "#e6edf3"
  terminal-surface: "#21262d"
  terminal-border: "#30363d"
  terminal-error: "#ff7b72"
  terminal-success: "#238636"
  terminal-muted: "#8b949e"

typography:
  body:
    fontFamily: "'Geist Variable', sans-serif"
    fontSize: 1rem
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: 0
  body-sm:
    fontFamily: "'Geist Variable', sans-serif"
    fontSize: 0.917rem
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: 0
  body-xs:
    fontFamily: "'Geist Variable', sans-serif"
    fontSize: 0.833rem
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: 0
  label:
    fontFamily: "'Geist Variable', sans-serif"
    fontSize: 0.75rem
    fontWeight: 500
    lineHeight: 1
    letterSpacing: 0.05em
  button:
    fontFamily: "'Geist Variable', sans-serif"
    fontSize: 1rem
    fontWeight: 500
    lineHeight: 1
    letterSpacing: 0

rounded:
  none: 0px
  sm: 2px

spacing:
  xs: 0.25rem
  sm: 0.5rem
  md: 0.75rem
  base: 1rem
  lg: 1.5rem
  xl: 2rem
  panel-header: 2.5rem
  toolbar: 2.333rem

components:
  button-default:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.primary-foreground}"
    typography: "{typography.button}"
    rounded: "{rounded.none}"
    padding: 0.25rem 0.625rem
    height: 2rem
  button-default-hover:
    backgroundColor: "{colors.primary}/80"
  button-outline:
    backgroundColor: "{colors.background}"
    textColor: "{colors.foreground}"
    typography: "{typography.button}"
    rounded: "{rounded.none}"
    padding: 0.25rem 0.625rem
    height: 2rem
  button-outline-hover:
    backgroundColor: "{colors.muted}"
  button-ghost:
    backgroundColor: transparent
    textColor: "{colors.muted-foreground}"
    typography: "{typography.button}"
    rounded: "{rounded.none}"
    padding: 0.25rem 0.5rem
    height: 2rem
  button-ghost-hover:
    backgroundColor: "{colors.muted}"
    textColor: "{colors.foreground}"
  button-destructive:
    backgroundColor: transparent
    textColor: "{colors.destructive}"
    typography: "{typography.button}"
    rounded: "{rounded.none}"
    padding: 0.25rem 0.625rem
    height: 2rem
  button-xs:
    height: 1.5rem
    padding: 0.125rem 0.5rem
  button-sm:
    height: 1.75rem
    padding: 0.125rem 0.625rem
  button-icon:
    size: 2rem
    rounded: "{rounded.none}"
  card:
    backgroundColor: "{colors.card}"
    textColor: "{colors.card-foreground}"
    rounded: "{rounded.none}"
    borderColor: "{colors.border}"
  sidebar:
    backgroundColor: "{colors.background}"
    textColor: "{colors.foreground}"
    borderColor: "{colors.border}"
  session-row:
    height: 1.75rem
    backgroundColor: transparent
    textColor: "{colors.muted-foreground}"
    typography: "{typography.body-xs}"
    rounded: "{rounded.sm}"
  session-row-active:
    backgroundColor: "{colors.muted}"
    textColor: "{colors.foreground}"
  panel-header:
    height: "{spacing.panel-header}"
    backgroundColor: "{colors.background}"
    borderColor: "{colors.border}"
  terminal:
    backgroundColor: "{colors.terminal-bg}"
    textColor: "{colors.terminal-fg}"
    padding: 0.333rem 0.333rem 0
  terminal-overlay:
    backgroundColor: "{colors.terminal-surface}"
    borderColor: "{colors.terminal-border}"
    textColor: "{colors.terminal-fg}"
    rounded: "{rounded.none}"
    padding: 0.75rem 1rem
---

## Overview

tillerd is a desktop developer tool — a local AI session manager with a
terminal-first surface. The entire UI is built around one idea: get out of
the way of the terminal. Chrome is minimal, flat, and compact. Every panel,
header, and sidebar is scaffolding for the thing that matters — the running
agent output.

The palette derives from the VSCode 2026 theme. In dark mode the canvas is a
near-black anthracite ({colors.background} #191a1b) with a steel-blue accent
({colors.primary} #297aa0). In light mode the canvas lifts to an off-white
({colors.background-light} #fafafd) with a saturated blue accent
({colors.primary-light} #0069cc). Both modes share identical semantic roles —
the same token names apply regardless of theme.

**Key characteristics:**
- Zero border radius — every surface is square. The tool should feel
  engineered, not rounded and approachable.
- Borders, never shadows. Elevation is expressed with 1px border lines, not
  drop shadows or blurs.
- 12px root. All rem measurements compute from 12px. Density is
  non-negotiable — screen real estate belongs to the terminal.
- Terminal is always dark. The terminal pane uses a hardcoded GitHub-dark
  palette ({colors.terminal-bg} #0d1117) regardless of host theme.
- Monochrome with one accent. The blue primary drives all interactive
  affordances — links, focus rings, active states. Everything else is neutral.
- Geist Variable for all text. Monospace-adjacent, designed for developer
  tools.

## Colors

### Background & Surface
- **Background** ({colors.background} dark / {colors.background-light} light):
  Main page floor. Near-black in dark mode, barely-off-white in light. Never
  pure black or pure white.
- **Card** ({colors.card}): Cards and elevated surfaces sit one step lighter
  than the background — #202122 in dark, pure #ffffff in light.
- **Popover** ({colors.popover}): Matches card; used for dropdowns and overlays.

### Primary Accent
- **Primary** ({colors.primary} dark / {colors.primary-light} light): The
  single interaction color. Steel blue in dark, saturated blue in light. Every
  interactive CTA, focus ring, and active indicator uses primary. Used
  scarcely — most screens are monochrome with one primary moment.
- **Primary Foreground** ({colors.primary-foreground}): Always #ffffff.

### Neutral Scale
- **Foreground** ({colors.foreground}): Main text. #bfbfbf in dark, #202020
  in light. Not pure white/black — reduces harshness.
- **Muted Foreground** ({colors.muted-foreground}): Secondary text, labels,
  inactive states. #8c8c8c dark / #606060 light.
- **Muted** ({colors.muted}): Hover and active state backgrounds. #2c2d2e
  dark / #eaeaea light.
- **Secondary** ({colors.secondary}): Slightly lighter than background;
  secondary button fills.
- **Accent** ({colors.accent}): Same value as muted; non-primary hover states.

### Borders & Input
- **Border** ({colors.border}): 1px separator lines everywhere — #2a2b2c dark,
  #e4e5e6 light.
- **Input** ({colors.input}): Form input backgrounds, slightly lighter than muted.
- **Ring** ({colors.ring}): Focus ring — matches primary in light, #3994bc
  (slightly lighter than primary) in dark.

### Semantic
- **Destructive** ({colors.destructive}): #f48771 in dark (salmon), #ad0707
  in light (deep red).

### Terminal (hardcoded, theme-independent)
- **Terminal Background** ({colors.terminal-bg} #0d1117): GitHub dark canvas.
  Fixed regardless of host theme.
- **Terminal Foreground** ({colors.terminal-fg} #e6edf3): Primary terminal text.
- **Terminal Surface** ({colors.terminal-surface} #21262d): Overlay backgrounds
  inside the terminal (crash notification, etc.).
- **Terminal Border** ({colors.terminal-border} #30363d): Borders within
  terminal overlays.
- **Terminal Error** ({colors.terminal-error} #ff7b72): Session-ended and error
  messaging.
- **Terminal Success** ({colors.terminal-success} #238636): Resume/confirm
  action fills.
- **Terminal Muted** ({colors.terminal-muted} #8b949e): Inactive/dismiss text.

## Typography

Font: **Geist Variable** (`@fontsource-variable/geist`). Single family for
all UI text — headings and body both use Geist. No dedicated heading family.
Fallback: `sans-serif`.

Root font size: **12px**. All rem values compute from 12px. This is
intentional and load-bearing — dense developer tool, not a content site.

| Token | Size (rem) | px | Weight | Use |
|---|---|---|---|---|
| body | 1rem | 12px | 400 | Default prose, panel content |
| body-sm | 0.917rem | ~11px | 400 | Sidebar items, secondary text |
| body-xs | 0.833rem | ~10px | 400 | Session row labels, tight contexts |
| label | 0.75rem | 9px | 500 | Section headings (uppercase + tracking) |
| button | 1rem | 12px | 500 | All button text |

**Principles:**
- No heading type scale. Headings use body weight/size variants, not a
  distinct family or large size.
- Labels use uppercase + `letter-spacing: 0.05em` — the only place tracking
  appears.
- Sub-12px sizes (`body-xs`, `label`) are intentional — density over
  readability at large sizes.
- No italic or heavy weights in the UI.

## Layout

The app occupies the full viewport (`h-dvh`, `overflow: hidden`). No
scrolling at the shell level — each panel manages its own overflow.

**Panel system:** Resizable split panes (horizontal and vertical). Sidebar
lives in a fixed-width left panel; the remainder is a resizable panel group
for terminal, diff, and agent views.

**Spacing scale** (base: 12px):

| Token | rem | px |
|---|---|---|
| xs | 0.25rem | 3px |
| sm | 0.5rem | 6px |
| md | 0.75rem | 9px |
| base | 1rem | 12px |
| lg | 1.5rem | 18px |
| xl | 2rem | 24px |

**Chrome dimensions:**
- Panel header: {spacing.panel-header} = 2.5rem (30px)
- Toolbar: {spacing.toolbar} = 2.333rem (28px)
- Button default: h = 2rem (24px)
- Button sm: h = 1.75rem (21px)
- Button xs / icon: h = 1.5rem (18px)

**Sidebar:** Full-height, scrollable session list. Horizontal padding: 0.75rem
(9px). Session rows: 1.75rem (21px) height, `gap-px` between items.

**Grid:** No fixed column grid. Panel widths are user-resizable; no max-width
container.

## Elevation & Depth

No shadows. Depth hierarchy is expressed through:

1. **Color steps** — card ({colors.card}) sits one stop lighter than
   background ({colors.background}).
2. **1px borders** — every surface boundary uses `border: 1px solid
   {colors.border}`.
3. **Tonal muting** — inactive/secondary content uses {colors.muted-foreground}
   instead of {colors.foreground}.

The terminal overlay (crash notification) is the only absolutely-positioned
element that reads as elevated — it uses {colors.terminal-surface} background
+ {colors.terminal-border} 1px border. No shadow.

## Shapes

`--radius: 0rem`. All corners are square. No border radius in the main shell,
sidebar, panels, or buttons.

**Exception:** A single `rounded-sm` (2px) appears on session nav rows and
icon-sized action buttons within the sidebar. These are the only rounded
elements and the radius is barely perceptible. It exists to prevent
hard pixel-corners on micro-interactive targets, not to soften the aesthetic.

**Resizable handles:** Active state expands to 4px for usability. Otherwise
invisible.

## Components

**`button-default`** — Primary CTA. Background {colors.primary}, text
{colors.primary-foreground}, height 2rem, padding 0.25rem × 0.625rem, zero
radius. Hover: primary/80 opacity.

**`button-outline`** — Secondary action. Background {colors.background},
{colors.border} 1px border. Hover: background shifts to {colors.muted}.

**`button-ghost`** — Tertiary / icon-adjacent. Transparent background,
{colors.muted-foreground} text. Hover: {colors.muted} fill + {colors.foreground}
text. Used extensively in sidebar for small action targets.

**`button-destructive`** — Transparent with {colors.destructive} text. Hover:
destructive/20 tinted background.

**`button-xs`** — h-6 (18px). Used for sidebar inline actions (archive, new
session, new project). `rounded-sm` override in sidebar context.

**`button-icon`** — Square, 2rem × 2rem. Icon-only. Ghost variant most common.

**`card`** — Background {colors.card}, zero radius, {colors.border} 1px border.
No shadow.

**`sidebar`** — Full-height, {colors.background} fill (matches page floor — no
elevation). Right edge: 1px {colors.border}.

**`session-row`** — Height 1.75rem, full width, `rounded-sm` (2px). Inactive:
{colors.muted-foreground} text, transparent background. Active: {colors.muted}
background, {colors.foreground} text. Status dot: 6px `rounded-full`,
`bg-emerald-500/80` — the only non-neutral color in the sidebar.

**`session-label` (section heading)** — 0.75rem, weight 500, uppercase,
letter-spacing 0.05em, {colors.muted-foreground}/70. Pure label, no interactive
state.

**`panel-header`** — Height 2.5rem, {colors.background} fill, bottom 1px
{colors.border}. Contains panel tab/title and panel action buttons.

**`terminal`** — Hardcoded dark canvas {colors.terminal-bg}, xterm.js renderer.
Padding 0.333rem on sides and top, 0 at bottom. Terminal manages its own scroll.

**`terminal-overlay`** — Crash/session-end notification. Absolute bottom-center,
{colors.terminal-surface} background, {colors.terminal-border} 1px border,
zero radius. Contains Resume ({colors.terminal-success} fill) and Dismiss
(ghost with {colors.terminal-border} border) buttons.

**`host-status-badge`** — Fixed bottom-right, `bg-black/60`, height 1.5rem,
`font-mono`, 0.75rem text. States: booting (amber-500/amber-300), ready
(emerald-500/emerald-300), error (red-500/red-300). Pointer-events none —
informational only.

## Do's and Don'ts

- Do use {colors.border} 1px borders for all surface separation — never
  box shadows.
- Do use {colors.muted-foreground} for inactive text, labels, placeholders, and
  secondary actions. Reserve {colors.foreground} for active/primary content.
- Do keep button heights on the defined scale (xs: 18px, sm: 21px, default:
  24px, lg: 27px). No custom heights.
- Do use {colors.primary} for focus rings, active nav indicators, and the single
  primary CTA per view. One primary moment per screen.
- Don't add border radius beyond `rounded-sm` (2px). The zero-radius aesthetic
  is intentional — rounded corners break the editor feel.
- Don't introduce shadows (box-shadow, drop-shadow, filter blur). All depth is
  tonal or border-based.
- Don't mix the terminal palette ({colors.terminal-*}) into the shell UI.
  Terminal colors are GitHub-dark and do not match either theme.
- Don't use font sizes above 1rem (12px) in chrome elements. Compact density
  is load-bearing.
- Don't use the emerald status dot (`bg-emerald-500/80`) outside the
  session-row live indicator. It's the one semantic color break in an otherwise
  monochrome sidebar.

## Known Gaps

- Animation and transition timings are not documented. Tailwind `transition-all`
  with default duration (150ms) is used throughout; no named motion scale.
- Dark mode is the primary documented mode. Light mode token counterparts are
  listed but the prose does not describe light-mode-specific component
  appearances.
- Diff panel component (`DiffPanel`) styles not extracted — uses an external
  renderer whose token integration is not yet defined.
- Agent pane (`AgentPane`) styles not extracted — surface in flux (deferred to
  1.x per roadmap).
- No icon sizing scale documented. Icons are used at `size-3` (12px),
  `size-3.5` (14px), `size-4` (16px) contextually; no formal token.
