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

**Workbench regions (0.0.20):** the shell composes five chrome regions around
the panel-area content outlet — title bar (top, native window decorations,
toggle toolbar), activity bar (far-left icon strip switching sidebar views:
Sessions, Search, Commands, Templates), primary sidebar (hosts the active
view), bottom panel (tabbed: Logs, Notifications), and status bar (bottom:
service health + workspace/session context left; notification bell + settings
right). Sidebar and bottom panel are independently hideable and drag-resizable;
hidden regions occupy no space. Active view, visibilities, sizes, and the
bottom panel's active tab persist through the settings store. The activity
bar's active-view edge is the workbench's single primary-accent moment.

**Panel system:** Resizable split panes (horizontal and vertical) inside the
content outlet; panels bind surfaces by placement. Manager surfaces (settings
editor, logs route) render in the panel area as routes, not domain surfaces.

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

## Motion

Named transition scale. Theme-independent; defined in `app.css`. Token names are frozen at
0.0.6. Apply with Tailwind arbitrary values, e.g. `transition-colors duration-[var(--motion-base)]
ease-standard`.

| Token | value | Use |
|---|---|---|
| `--motion-instant` | 0ms | No transition (immediate state flips) |
| `--motion-fast` | 100ms | Hover/active feedback on small targets |
| `--motion-base` | 150ms | Default for color/background transitions |
| `--motion-slow` | 250ms | Larger surface changes (panel reveal) |
| `--ease-standard` | `cubic-bezier(0.2, 0, 0, 1)` | The single shell easing curve |

Prefer `transition-colors`/`transition-opacity` over `transition-all`. One easing curve only —
no bounce, no per-component custom timing.

## Icon Sizing

Three sizes cover the shell. Defined in `app.css`; token names frozen at 0.0.6. Apply with
`size-[var(--icon-md)]`.

| Token | rem | px | Use |
|---|---|---|---|
| `--icon-sm` | 0.75rem | 12px | Dense inline / sidebar action icons |
| `--icon-md` | 0.875rem | 14px | Default action icons |
| `--icon-lg` | 1rem | 16px | Panel headers / primary controls |

## Light Mode

Every color token has a light-mode counterpart (`light-2026.css`, `:root`) mirroring the dark set
(`.dark`); the shell renders identically structured in both. Light-mode specifics:

- **Canvas lifts:** background is off-white ({colors.background} `#fafafd`); cards are pure white
  ({colors.card} `#ffffff`) — the one-stop-lighter card elevation from dark mode inverts to
  one-stop-whiter.
- **Borders read darker than the canvas** ({colors.border} `#e4e5e6`) rather than lighter, keeping
  the 1px-border depth model intact without shadows.
- **Primary deepens** to {colors.primary} `#0069cc` for contrast on light surfaces; destructive
  deepens to `#ad0707`.
- **The terminal palette is exempt** — `terminal-*` tokens stay GitHub-dark in both themes, so the
  terminal canvas is dark even in light mode (by design; do not theme it).

## Contrast (WCAG AA)

Verified token pairings for every fg/bg combination actually used in chrome, both themes.
Thresholds: 4.5:1 normal text, 3:1 large text / UI components (WCAG 2 AA). Token *values* are
frozen at 0.0.6 — failures below are recorded as findings, not fixed by changing palette values.

| Pair (fg on bg) | Kind | Dark | Light |
|---|---|---|---|
| foreground / background | text | 9.48:1 PASS | 15.64:1 PASS |
| muted-foreground / background | text | 5.18:1 PASS | 6.04:1 PASS |
| foreground / card | text | 8.77:1 PASS | 16.29:1 PASS |
| muted-foreground / card | text | 4.80:1 PASS | 6.29:1 PASS |
| popover-foreground / popover | text | 8.77:1 PASS | 16.29:1 PASS |
| muted-foreground / popover | text | 4.80:1 PASS | 6.29:1 PASS |
| foreground / muted | text | 7.50:1 PASS | 13.54:1 PASS |
| muted-foreground / muted | text | 4.10:1 **FAIL** | 5.23:1 PASS |
| foreground / secondary | text | 8.35:1 PASS | 13.54:1 PASS |
| secondary-foreground / secondary | text | 8.35:1 PASS | 13.54:1 PASS |
| accent-foreground / accent | text | 11.79:1 PASS | 11.66:1 PASS |
| primary-foreground / primary | text | 4.79:1 PASS | 5.39:1 PASS |
| destructive / background | text | 7.10:1 PASS | 7.17:1 PASS |
| destructive / card | text | 6.57:1 PASS | 7.47:1 PASS |
| destructive / muted | text | 5.62:1 PASS | 6.21:1 PASS |
| ring / background | ui (3:1) | 5.09:1 PASS | 5.18:1 PASS |
| ring / card | ui (3:1) | 4.71:1 PASS | 5.39:1 PASS |
| border / background | ui (3:1) | 1.23:1 **FAIL** | 1.21:1 **FAIL** |

**Findings:**
- `muted-foreground` on `muted` fails AA in dark mode only (4.10:1, needs 4.5:1). Audited every
  usage in swept chrome: every static occurrence pairs `bg-muted` with `text-foreground` (the
  ghost-hover pattern always flips text to `foreground` on the same transition that applies
  `bg-muted`), so this pairing is never rendered at rest — recorded as a finding, no usage change
  needed. If a future component pairs `muted-foreground` text directly on a resting `bg-muted`
  surface in dark mode, it will fail AA and must use `foreground` instead.
- `border` on `background` fails the 3:1 UI-component threshold in both themes (1.2:1) — the
  1px border token is a low-contrast hairline by design (DESIGN.md: borders read as tonal
  separators, not shape-defining outlines). Per WCAG 1.4.11, this applies to graphical objects
  required to identify a UI component; treated as a design tradeoff for decorative/structural
  separators (panel edges, dividers) rather than a required affordance boundary. Where a border
  is the *only* affordance for an interactive control (e.g. `button-outline`), the control is
  never presented without accompanying text and a visible focus ring (`ring` token, which
  independently passes 3:1 above), so operability doesn't depend on the border being perceivable.
  Token value is frozen — no fix available without a palette change; recorded as a known finding.

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

- Diff panel component (`DiffPanel`) styles not extracted — uses an external
  renderer whose token integration is not yet defined.
- Agent pane (`AgentPane`) styles not extracted — surface in flux (deferred to
  1.x per roadmap).
