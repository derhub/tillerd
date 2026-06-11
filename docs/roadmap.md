# Roadmap

Status: `[WIP]` in progress · unlabeled = planned · `[HELP]` wants input.
Within a milestone, items are ordered most to least foundational.

> Status: 0.0.x, pre-release. Core components are scaffolded — PTY daemon, gate,
> desktop shell — but the app does not yet work end-to-end. 0.1.0 is the first
> complete release. See [CHANGELOG](../CHANGELOG.md).

---

## 0.0.1 — App opens, services run

The narrowest vertical slice: desktop opens, the daemon starts, the UI connects.
The gate is not in this path — a terminal streams UI → bridge → daemon → PTY, and
the host boots without it. Nothing renders yet; this milestone proves the
daemon-supervised startup chain reaches `ready` on a fresh launch.

- Daemon supervised startup — desktop adopt-or-spawns the daemon; the host does not
  assume one is already running.
- Bridge live — after ensure, `daemon_connect` succeeds and the byte bridge is open.
- UI loads — `useDesktopHost` reaches `ready`; app renders without errors (blank
  terminal is acceptable).

---

## 0.0.2 — Terminal works

A real terminal session opens and streams inside the desktop app.

- `DesktopTerminalPane` opens a PTY session, streams input and output through xterm.
- Session sidebar shows the active session.
- Works reproducibly on a fresh launch.

---

## 0.0.3 — Session model (terminal surface)

The session abstraction over the daemon's existing durability: the daemon already
snapshots, replays, and resumes; this milestone is the desktop layer that lists and
reconnects sessions across restarts.

- Session model, terminal surface (ADR-0020) — create, list, resume, close; the
  terminal surface reconnects to an existing session after a desktop restart.

---

## 0.1.0 — Works end-to-end

The first complete release: drive an agent in the desktop app, with hook events and
signed bundles. The terminal surface lands across 0.0.x; 0.1.0 adds the agent.

- Gate supervised startup — desktop adopt-or-spawns the gate (net-new: no gate
  supervisor exists today). Prerequisite for the hook-event path.
- Agent surface — agent pane renders, streams, and shows lifecycle and failure
  states (gate / daemon / agent down).
- Session model, agent surface (ADR-0020) — the agent surface added to the session
  model; create / list / resume / close across both terminal and agent surfaces.
- Hook-event path — desktop subscribes to the gate's hook fan-out and routes events
  to the right session / surface (agent status, activity, content).
- Desktop distribution — signed, notarized bundles (dmg / AppImage / deb) plus
  auto-update.
- Desktop end-to-end test — a visual test driving the real app: spawn a session,
  assert the terminal renders and streams.
- Release pipeline — versioned releases and generated changelogs via changesets.

---

## 0.2.0 — Standardize (before more services)

Make every long-lived service born the same way before the service count grows.
Uniform lifecycle, discovery, health, and storage so the next release can add
services without bespoke wiring each time.

- Single host — retire the server path; the desktop is the only thing that starts
  and owns sessions, and the orchestration domain (mint, register-before-spawn, env,
  adopt-or-spawn) moves out of the shell into a crate the shell calls.
- One service contract — every long-lived service born the same: lifecycle,
  discovery, health.
- Persistence model — formalize storage tiers; standardize durable state on SQLite
  (discovery files and ephemeral state excluded).
- One observability model — a single agent action traceable end-to-end across
  process hops.
- Drain-and-restart daemon upgrade — replace fd-handoff with a simpler
  planned-upgrade path. `[WIP]`
- Docs reconciliation — README and guides match the consolidated desktop-only
  architecture.
- Validate the extension point — pressure-test the adapter contract with a second
  agent before relying on it.
