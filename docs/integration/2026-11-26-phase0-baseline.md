# Phase 0 Baseline Report (2026-11-26)

## 1. Backups & Preservation (`P0.1`)
- `/var/tmp/firecraft-phase0-backup-20261126/` holds `experiment-dirty.patch`, `experiment-status.txt`, `firecraft-stash-{0,1,2}.patch`, `branch-shas.txt`.
- 4 local backup tags created: `backup/20261126/{main-local,experiment-merge,dev-roadmap,gate-repair}`.

## 2. CraftAnything Experiment (`P0.2`)
- Path: `/nas/Temp/repos/FireCraft-CraftAnything-Experiment` (`main` @ `6e9d7cb`, read-only).
- See `plan/phase0-experiment-inventory.md` for full module & UI delta table.

## 3. FireCraft Git & PRs (`P0.3`–`P0.4`)
- See `plan/phase0-git-inventory.md` and `plan/phase0-pr-reconciliation.md`.
- **PR #188:** Merge to `main` first (docs-only).
- **PR #187:** Retain shell split, CS6 panels, Phosphor icons (`72318ae7`).
- **PR #186:** DO NOT MERGE DIRECTLY (~118 MB binaries + `log/devlog.md`); cherry-pick clean commits only (`P1.3`–`P1.4`).
- **Worktree `feat/dev-roadmap-improvements` (`17438502`):** Cherry-pick startup, Windows GUI subsystem, Help links, T29–T31, `Tutorials.md`.

## 4. Issues, Releases, Wiki & Cloudflare (`P0.5`–`P0.6`)
- 31 open issues & 10 releases snapshotted in `plan/phase0-issues-releases-snapshot.json`.
- Live Wiki (`FireCraft.wiki.git`) cloned read-only to `/var/tmp/firecraft-wiki-audit` (14 pages + `_Sidebar.md`/`_Footer.md`).
- Cloudflare state recorded in `plan/phase0-cloudflare-audit.md`.

## 5. CI Health (`P0.7`)
### `gitforge_doctor.py`
```
rc=0
{
 "status": "healthy",
 "checked_at": "2026-11-26T02:09:58-0600",
 "summary": "17 checks, 0 degraded, 2 skipped",
 "checks": [
  {
   "check": "git_transport",
   "status": "ok",
   "detail": "ls-remote answered",
   "latency_seconds": 0.07,
   "url": "http://localhost:42782/mkinney/FireCraft.git"
  },
  {
   "check": "health:gitforge-api",
   "status": "ok",
   "detail": "HTTP 200"
  },
  {
   "check": "health:gitforge-ci",
   "status": "ok",
   "detail": "HTTP 200"
  },
  {
   "check": "unit:gitforge@api.service",
   "status": "ok",
   "detail": "state=active restarts=0 since=Thu 2026-11-12 09:13:30 CST"
  },
  {
   "check": "unit:gitforge@ci.service",
   "status": "ok",
   "detail": "state=active restarts=0 since=Sun 2026-11-22 23:42:43 CST"
  },
  {
   "check": "unit:gitforge@git-server.service",
   "status": "ok",
   "detail": "state=active restarts=0 since=Thu 2026-11-12 09:13:29 CST"
  },
  {
   "check": "unit:gitforge@runner.service",
   "status": "ok",
   "detail": "state=active restarts=0 since=Sun 2026-11-22 23:46:44 CST"
  },
  {
   "check": "unit:gitforge-local-registry.service",
   "status": "ok",
   "detail": "state=active restarts=0 since=Mon 2026-11-02 19:34:53 CST"
  },
  {
   "check": "db:read",
   "status": "ok",
   "detail": "mode=ro read 0.1ms",
   "latency_ms": 0.1
  },
  {
   "check": "db:latest-run",
   "status": "ok",
   "detail": "latest local run commit 90135d7a089b9041b8eb98d40ef791de89678323",
   "latest_commit_hash": "90135d7a089b9041b8eb9
```
### Fedora Runners
```
rc=4
inactive
```
- **GitHub Actions Root Cause:** `.github/workflows/ci-cross.yml` has a 39-char `actions/checkout` SHA + unpinned `actions/cache@v4`; `.github/workflows/ci.yml` has a 44-char corrupted `actions/cache` SHA. Both will be fixed in `P1.2`.

## 6. Proposed Phase 1 Execution Order
1. Merge PR #188 to `main` (or rebase onto it).
2. Create `integration/firecraft-1.0` in `/var/tmp/firecraft-int-1.0` from `origin/main`.
3. Fix CI SHAs (`P1.2`), salvage clean PR #186 commits (`P1.3`–`P1.4`), integrate PR #187 (`P1.5`), integrate `17438502` (`P1.6`), reconcile stashes & `6a78399f` (`P1.7`).
