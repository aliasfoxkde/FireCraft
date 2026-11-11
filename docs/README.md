# FireCraft documentation

This page is the documentation map. Start with the overview that matches your goal, then follow its links into the detailed reference.

## Choose a path

### I want to use FireCraft

1. [Usage](USAGE.md): install/build, CLI examples, keyboard shortcuts, and browser embedding.
2. [Features](FEATURES.md): implemented surfaces and their verification gates.
3. [WebOS embedding](WEBOS.md): host page contract and browser bridge.

### I want to understand project status

1. [Progress](PROGRESS.md): current verified snapshot and release/build evidence.
2. [Task board](TASKS.md): the single current list of active, queued, blocked, and completed tasks.
3. [Next](NEXT.md): detailed phased roadmap and acceptance criteria.
4. [CS6 parity matrix](CS6-PARITY.md): the user-visible inventory — every CS6 workflow and UI area, classified with evidence.
5. [Published Wiki](https://github.com/aliasfoxkde/FireCraft/wiki): concise user and contributor navigation; the source is [`wiki/Home.md`](wiki/Home.md).

### I want to contribute or use an AI agent

1. [`../AGENTS.md`](../AGENTS.md): repository rules, clean-room boundary, commands, and workflow.
2. [Handoff](HANDOFF.md): short orientation and session closeout procedure.
3. [Architecture and format plan](PLANNING.md): technical design and file format detail.
4. [Decisions](decisions/): durable design choices; [research](research/): observed format behavior.

## Reference shelves

| Level | Documents | Use them for |
|---|---|---|
| Overview | [README](../README.md), this page, [Wiki](wiki/Home.md) | Project purpose and navigation |
| User guide | [USAGE](USAGE.md), [FEATURES](FEATURES.md), [WEBOS](WEBOS.md) | Operating the app and understanding its surface |
| Current state | [PROGRESS](PROGRESS.md), [TASKS](TASKS.md), [NEXT](NEXT.md), [CS6-PARITY](CS6-PARITY.md) | What is true now and what comes next |
| Engineering | [HANDOFF](HANDOFF.md), [PLANNING](PLANNING.md), [TOOLCHAINS](TOOLCHAINS.md), [RELEASING](RELEASING.md) | Building, architecture, and release work |
| Evidence | [decisions](decisions/), [research](research/), [generated parity and scorecard](planning/generated/) | Specific technical claims and their basis |
| History | [SESSIONS](SESSIONS.md), [HISTORY](HISTORY.md), [closed milestones](planning/firecraft/), [release notes](releases/) | How the project arrived here; not live task status |

## Source-of-truth rules

- `docs/PROGRESS.md` is the dated status snapshot.
- `docs/TASKS.md` is the live task ledger. Detailed acceptance criteria belong in `docs/tasks/` and `docs/NEXT.md`.
- `docs/SESSIONS.md` and `docs/HISTORY.md` are historical. Do not infer current work from an old entry.
- `docs/planning/generated/` is generated output; use its generator rather than editing derived counts by hand.
- `source/` and Adobe-derived local extracts are reference-only and must never be committed.
