# Roadmap

Where tillerd is heading. Work is grouped as **Strategic/Foundational**,
**Important**, or **Nice to have**; at least one Strategic item lands per release.

Status: `[WIP]` in progress · unlabeled = planned · `[HELP]` wants input.

> Status: 0.0.x, pre-release. Core components are scaffolded — PTY daemon, gate,
> desktop shell — but the app does not yet work end-to-end. 0.1.0 is the first
> working release. See [CHANGELOG](../CHANGELOG.md).

---

## 0.1.0 — Solid foundation

The first working release: make the desktop app run end-to-end on a uniform
foundation. The pieces exist but do not yet work as a whole, and the foundation is
uneven — a duplicated host path, per-service lifecycle, no owner for startup.

### Strategic/Foundational

- Working desktop UI — open and drive surfaces in the desktop app: terminal and
  agent panes render, stream, and show lifecycle + failure states (gate / daemon /
  agent down).
- Session model (ADR-0020) — a session is a workspace of many surfaces (terminal,
  agent, …), kind-agnostic; create / list / resume / close, survives restart.
  0.1 ships terminal and agent surfaces.
- Hook-event path — the desktop subscribes to the gate's hook fan-out and routes
  events to the right session / surface (agent status, activity, content).
- Single host — retire the server path; the desktop is the only thing that starts
  and owns sessions, and the orchestration domain (mint, register-before-spawn,
  env, adopt-or-spawn) moves out of the shell into a crate the shell calls.
- One service contract — every long-lived service born the same: lifecycle,
  discovery, health.
- Supervised startup — no service assumes another is already running; every
  dependency is adopt-or-spawned.
- Persistence model — formalize storage tiers; standardize durable state on SQLite
  (discovery files and ephemeral state excluded).
- Desktop distribution — signed, notarized bundles (dmg / AppImage / deb) plus
  auto-update.

### Important

- Desktop end-to-end test — a visual test driving the real app: spawn a session,
  assert the terminal renders and streams.
- One observability model — a single agent action traceable end-to-end across
  process hops.
- Drain-and-restart daemon upgrade — replace fd-handoff with a simpler
  planned-upgrade path. `[WIP]`
- Release pipeline — versioned releases and generated changelogs via changesets.
- Docs reconciliation — README and guides match the consolidated desktop-only
  architecture.

### Nice to have

- Validate the extension point — pressure-test the adapter contract with a second
  agent before relying on it.
