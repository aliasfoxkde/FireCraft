# 0094 — The changelog is additive: the committed file is history, the log is only new material

**Status:** accepted
**Date:** 2026-11-11
**Issue:** #121
**Supersedes:** nothing

## Context

`xtask changelog` regenerates `docs/CHANGELOG.md` from the conventional-commit
log. The regeneration used to be *log-only*: the file was written from
`git log` on every run.

That is wrong, and it was wrong in a way that silently rewrote history.
A PR that lands as a **squash merge** enters `main`'s history as a single
commit. The branch's own commits — the ones the release notes are actually
about — are unreachable from `HEAD` and never appear in `git log` again. A
regeneration therefore *deleted* every entry that had arrived by squash
merge, and the next run deleted them again.

The train merge (`658f015`, PR #107) landed 17 commits as a squash. The
regeneration that followed removed all 17 entries from the committed file.
Nothing failed: the verb wrote the truncated file, the gate read the exit
status of its last command, and the changelog simply became shorter. The
first person to notice is whoever runs the gate locally and sees a scary
diff — or, worse, whoever reads the release notes.

Two properties make it worse:

1. **The damage is not detectable from the tool's exit status.** The
   generator succeeded; it produced a file that was missing content.
2. **The file was the only copy.** The entries that could not be
   regenerated existed in exactly one place — the committed file — and the
   verb's job was to overwrite it.

## Decision

**The committed file is the historical record; the log supplies only what
is new.** The generation is additive.

- The committed file's entries are parsed and **kept**. The log's entries
  are added, newest first, and an entry the file already records is never
  repeated. Identity is the commit hash the entry line carries.
- **Released sections are history.** A `## <version>` section is preserved
  verbatim, and the commits it records are not offered to the unreleased
  block again. The recorded-hash set spans the whole file for that reason.
- **The unreleased block is the only generated region.** `## Unreleased`
  and its `### <Section>` subsections are regenerated; everything above it
  that is a released section is copied through untouched.

Two release verbs bracket the file:

- `xtask changelog --check` — the freshness gate. It fails when the
  committed file is not what the generator produces, and names the first
  differing line. A release must not ship notes that omit the commits it
  releases. The tip commit is exempt from the comparison, and it has to
  be: a file cannot record the hash of the commit that writes it. The
  release flow therefore regenerates the notes in the release commit
  itself, and the exemption covers exactly that one commit.
- `xtask changelog --cut <version>` — freezes the unreleased block into a
  released `## <version>` section and starts an empty unreleased block.
  The version must be the one the workspace carries, so a section heading
  and the "Current version" line can never disagree.

## Consequences

- The regeneration is idempotent: a second run over an unchanged tree
  produces the same file.
- The gate still **writes** the file. It is a step of `xtask ci`, so a
  gate run leaves a regenerated copy in the working tree. With the
  additive rule that copy only ever *adds* entries — the tree is never
  left with a truncated changelog. `--check` is the non-mutating
  question ("is the committed file current?"), and it is a release gate
  rather than a CI gate: making CI demand equality would fail every
  branch that has not regenerated its notes, which is every branch.
- `--check` needs git, like the generator, so it cannot run in the CI
  image. The stamp half stays git-free and is what CI runs (decision
  0086).
- The unreleased block grows until a release cuts it. That is the
  intended shape: the block is the work since the last release, and the
  cut is what makes it a release.
- The first cut freezes all history into a `## <version>` section. After
  it, the file carries every entry exactly once, and no future
  regeneration can remove one.
