---
description: Dev driver — resume the pending OpenSpec change (status + what's left) or prepare the next roadmap version
argument-hint: "[version e.g. 0.0.2]  (optional; defaults to auto-detect)"
allowed-tools: Bash, Read, Glob, Grep, EnterWorktree
---

tillerd dev driver. Preflight the worktree, then **resume** the active change or **prepare** the next.
Sets up the worktree; otherwise read-only — reports status + the roadmap edit, never edits artifacts.

## State (pre-loaded)

- Branch / worktree: !`echo "$(git rev-parse --abbrev-ref HEAD) @ $(git rev-parse --show-toplevel)"`
- Worktrees: !`git worktree list | head -8`
- Tree: !`git status --short | head -10 || echo clean`
- Changes: !`openspec list 2>/dev/null | head -20 || ls -1 openspec/changes | grep -v '^archive$'`
- Archived: !`ls -1 openspec/changes/archive 2>/dev/null | head -12 || echo NONE`
- Bindings: !`grep -rn 'Implements roadmap' openspec/changes --include=proposal.md 2>/dev/null | grep -v '/archive/' | head`
- Next roadmap gap: !`awk '/^### [0-9]/{h=$0} /^- \[ \]/{if(h){print h; h=""} print}' docs/roadmap.md | head -16`
- Arg: "$ARGUMENTS"

## 1. Preflight — worktree (first)

This command authorizes `EnterWorktree`; build every change in its own worktree, never main.

- **Focus + branch:** `$ARGUMENTS` version → it; else active change dir (under `openspec/changes/` ≠ `archive/`) → RESUME; else next roadmap gap → PREPARE. `<id>` = change kebab-id, branch `feature/<id>`.
- **Place the session:** already there → continue · worktree exists in `git worktree list` → `EnterWorktree({path})` · RESUME, branch exists, no worktree → `git worktree add .claude/worktrees/<id> feature/<id>` + `EnterWorktree({path})` · PREPARE, no branch → `EnterWorktree({name:"feature/<id>"})` (off `main`).
- Entered and `.env` missing → run `tools/setup-dev.sh`.
- **Blocked (flag, don't force):** artifacts uncommitted on main → `/commit` onto `feature/<id>` first · branch already checked out in main → switch main back to `main` first.

## 2. RESUME

- Read `proposal.md` + `tasks.md`; note artifacts present (proposal / specs / design / tasks).
- **Version:** the proposal `Implements roadmap X.Y.Z` marker. Missing → flag `⚠ binding missing`, then guess by title (say so).
- **Tasks:** done `- [x]` / total; report the Verification group (heading ends in `Verification`, e.g. `## 7. Verification`) separately — none → flag `⚠ no Verification group`.
- **Next step:** missing artifact (proposal→specs→design→tasks) → `/opsx:continue` · impl tasks partial → `/opsx:apply` · impl done but Verification not `[x]` → `/opsx:apply` · all `[x]` incl Verification → `/opsx:verify` · verified → `/opsx:archive` (user-only). Uncommitted work → `/commit`.

## 3. PREPARE

- **Reconcile first:** a `- [ ]`/`[WIP]` version whose change sits in `archive/` → don't prepare; report the ROADMAP EDIT to mark it done, stop.
- First `### 0.x.y` (top-down) with any `- [ ]` (unless `$ARGUMENTS`). Summarize title + one-line demoable outcome + unchecked bullets; constraining ADRs (scan `ADR-00xx`; default 0020–0023); schema `spec-driven` (small → `minimalist`/direct).
- Emit a change description for `/opsx:propose` (one-shot) or `/opsx:new` (step-by-step): scope bullets verbatim + a binding line `Implements roadmap X.Y.Z — "<title>"` + **Acceptance criteria** from the demoable outcome (→ a real `## Verification` group).

## Output (tight)

```
MODE: resume | prepare
VERSION: <0.x.y — title>   CHANGE: <id|none>   (source: marker|guessed)
WORKTREE: <path>  (entered | created | already here | ⚠ blocked: artifacts/branch on main)

DONE: <checked / implemented>            (resume)
LEFT: <unchecked / remaining>
VERIFY: <N/M checks; is /opsx:verify the gate next?>   (resume)

ROADMAP EDIT (report only — user applies):
- <"mark 0.0.1 header [WIP]" | "after archive: mark 0.0.1 bullets [x], drop [WIP]" | none>

NEXT STEP: <one command: /opsx:apply | /opsx:continue | /opsx:verify | /opsx:propose | /commit>
<one line why>
```

## Rules

- Sets up the worktree only; otherwise **never edits** code/specs/tasks/`docs/roadmap.md` — report edits under ROADMAP EDIT.
- Version is read from the proposal marker, never guessed silently.
- Roadmap DONE (`[x]` bullets, drop `[WIP]`) **only after** archive + Verification green + `/opsx:verify`; while applying → header `[WIP]`.
- Never auto-run `/opsx:apply` `/opsx:sync` `/opsx:archive`. Honor ADRs. One version at a time, in order; flag scope drift.
