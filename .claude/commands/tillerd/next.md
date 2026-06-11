---
description: Dev driver — resume the pending OpenSpec change (status + what's left) or prepare the next roadmap version
argument-hint: "[version e.g. 0.0.2]  (optional; defaults to auto-detect)"
allowed-tools: Bash, Read, Glob, Grep
---

tillerd dev driver. Pick a mode — **resume** a pending change or **prepare** the next — then
report a tight status, the roadmap edit it implies, and ONE next step. **Read-only: report, never edit.**

## State (pre-loaded)

- Branch / worktree: !`echo "$(git rev-parse --abbrev-ref HEAD) @ $(git rev-parse --show-toplevel)"`
- Worktrees: !`git worktree list | head -8`
- Tree: !`git status --short | head -10 || echo clean`
- Changes: !`openspec list 2>/dev/null | head -20 || ls -1 openspec/changes | grep -v '^archive$'`
- Archived: !`ls -1 openspec/changes/archive 2>/dev/null | head -12 || echo NONE`
- Bindings: !`grep -rn 'Implements roadmap' openspec/changes --include=proposal.md 2>/dev/null | grep -v '/archive/' | head`
- Next roadmap gap: !`awk '/^### [0-9]/{h=$0} /^- \[ \]/{if(h){print h; h=""} print}' docs/roadmap.md | head -16`
- Arg: "$ARGUMENTS"

## Mode

`$ARGUMENTS` names a version → focus it. Else active change (dir under `openspec/changes/` ≠ `archive/`) → RESUME. Else → PREPARE.

## RESUME

- Read the change's `proposal.md` + `tasks.md`; note artifacts present (proposal / specs / design / tasks).
- **Version:** from the proposal `Implements roadmap X.Y.Z` marker. Missing → flag `⚠ binding missing` and only then guess by title (say so).
- **Tasks:** done `- [x]` / total. The **Verification group** (heading ends in `Verification`, e.g. `## 7. Verification`) is reported separately. No such group → flag `⚠ no Verification group`.
- **Ladder → next step:**
  - artifact missing for schema (proposal→specs→design→tasks) → `/opsx:continue`
  - planning done, impl tasks partial → `/opsx:apply`
  - impl tasks `[x]`, Verification not `[x]` → `/opsx:apply` (finish the checks)
  - all `[x]` incl Verification → `/opsx:verify` (gate)
  - verify passed → `/opsx:archive` (user-only)
- Hygiene: on a `feature/*` branch + a dedicated worktree (not the main worktree / `main`)? Uncommitted work to `/commit`?

## PREPARE

- **Reconcile first:** a version still `- [ ]`/`[WIP]` whose change is in `archive/` → don't prepare; report the ROADMAP EDIT to mark it done, stop.
- Pick the first `### 0.x.y` (top-down) with any `- [ ]` (unless `$ARGUMENTS` overrides). Summarize: title, one-line demoable outcome, unchecked bullets.
- ADRs that constrain it (scan bullets for `ADR-00xx`; default honor 0020–0023). Schema: `spec-driven` default; `minimalist`/direct for small ones.
- **Worktree (default isolation):** propose `git worktree add ../tillerd-<id> -b feature/<id> main` (`<id>` = change kebab-id), then work from that worktree.
- Emit a ready **change description** for `/opsx:propose` (one-shot) or `/opsx:new` (step-by-step), containing: the scope bullets verbatim; a binding line `Implements roadmap X.Y.Z — "<title>"`; and **Acceptance criteria** from the demoable outcome (so `tasks.md` gets a real `## Verification` group).

## Output (tight)

```
MODE: resume | prepare
VERSION: <0.x.y — title>   CHANGE: <id|none>   (source: marker|guessed)
LOCATION: <branch> @ <worktree>   (flag if main / main-worktree / dirty)

DONE: <checked / implemented>            (resume)
LEFT: <unchecked / remaining>
VERIFY: <N/M checks; is /opsx:verify the gate next?>   (resume)

ROADMAP EDIT (report only — user applies):
- <"mark 0.0.1 header [WIP]" | "after archive: mark 0.0.1 bullets [x], drop [WIP]" | none>

NEXT STEP: <one command: /opsx:apply | /opsx:continue | /opsx:verify | /opsx:propose | git worktree add ...>
<one line why>
```

## Rules

- **Read-only** — never edit code/specs/tasks/`docs/roadmap.md`; report the edit under ROADMAP EDIT, user applies.
- **Version binding read, not guessed** — from the proposal marker; absent → flag + mark guessed.
- **Roadmap lifecycle (reported):** applying → header `[WIP]`; DONE (`[x]` bullets, drop `[WIP]`) **only after** archive + Verification green + `/opsx:verify` passed.
- **Every change needs a Verification group** (heading ends in `Verification`) of runnable acceptance checks; missing/not-green → not done, surface it.
- **Worktree-first** — a change is built in its own worktree off `main`; flag work happening in the main worktree.
- Never auto-run `/opsx:apply` `/opsx:sync` `/opsx:archive` — surface as next step. Honor ADRs. One version at a time, in order; call out scope drift.
