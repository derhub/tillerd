---
description: Dev driver — resume the pending OpenSpec change (status + what's left) or prepare the next roadmap version
argument-hint: "[version e.g. 0.0.2]  (optional; defaults to auto-detect)"
allowed-tools: Bash, Read, Glob, Grep
---

You are the tillerd development driver. Decide between TWO modes — **resume** a pending
change, or **prepare** the next one — then report a tight status and ONE clear next step.

## State (pre-loaded)

- Branch: !`git branch --show-current`
- Working tree: !`git status --short | head -20 || echo clean`
- Active changes (exclude archive): !`ls -1 openspec/changes 2>/dev/null | grep -v '^archive$' || echo NONE`
- OpenSpec change list: !`openspec list 2>/dev/null | head -30 || echo "openspec CLI unavailable — use the filesystem"`
- Roadmap next-unchecked scan: !`grep -nE '^### [0-9]|- \[ \]' docs/roadmap.md | head -40`
- Argument (target version, optional): "$ARGUMENTS"

## Procedure

1. **Pick the change to focus.**
   - If `$ARGUMENTS` names a version, focus that.
   - Else if an active change exists (a dir under `openspec/changes/` other than `archive/`), focus it (RESUME mode).
   - Else go to PREPARE mode.

2. **RESUME mode — there is an active change.**
   - Read its `proposal.md` (intent) and `tasks.md` (the checklist). Also note which artifacts exist: `proposal.md`, `specs/`, `design.md`, `tasks.md`.
   - Count tasks: done = `- [x]`, left = `- [ ]`. Compute a simple done/total.
   - Map it back to the roadmap version it implements (match by title/scope in `docs/roadmap.md`).
   - Determine the phase and the next step:
     - Planning incomplete (an expected artifact is missing for the `spec-driven` schema: proposal → specs → design → tasks) → next step is **`/opsx:continue`**.
     - Planning complete, tasks unstarted or partial → next step is **`/opsx:apply`**.
     - All tasks `- [x]` → next step is **`/opsx:verify`**, then **`/opsx:archive`** (archive is user-only — never auto-run it).
   - Check git hygiene: are we on a `feature/*` branch (not `main`)? Any uncommitted work that should be committed (`/commit`) first?

3. **PREPARE mode — no active change.**
   - In `docs/roadmap.md`, find the next version to build: the FIRST `### 0.x.y` (top-down) that still has any `- [ ]` items, unless `$ARGUMENTS` overrides.
   - Summarize that version: its title and its unchecked bullets (the scope).
   - Identify which ADRs constrain it (scan the bullets for `ADR-00xx`; default to honoring ADR-0020–0023).
   - Recommend a schema: `spec-driven` (default) for multi-piece versions; `minimalist` or direct-implement for small ones (e.g. wire-types, docs).
   - Propose a branch name: `feature/<kebab-desc>` off `main`.
   - Produce a ready-to-run **change description** (the scope bullets, verbatim) the user can hand to `/opsx:propose` (one-shot) or `/opsx:new` (step-by-step).

## Output format (keep it tight)

```
MODE: resume | prepare
VERSION: <0.x.y — title>   CHANGE: <change-id or "none yet">
BRANCH: <branch>  (flag if on main / dirty tree)

DONE:
- <checked items / what's implemented>     (resume only)
LEFT:
- <unchecked items / what remains>

NEXT STEP: <single command to run, e.g. `/opsx:apply`, `/opsx:continue`,
            `/opsx:propose`, or `git checkout -b feature/...`>
<one line why>
```

## Rules

- Do NOT implement or edit code, specs, or tasks here — this command only reports status and the next step.
- Never auto-run `/opsx:apply`, `/opsx:sync`, or `/opsx:archive` — surface them as the next step; the user runs them.
- Honor the ADRs; do not re-decide architecture.
- One version at a time, in roadmap order. If the active change's scope has drifted past its version, say so.
