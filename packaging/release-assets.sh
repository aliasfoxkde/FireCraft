#!/usr/bin/env bash
# release-assets.sh — stage the downloaded release artifact tree and build
# the argument vector for `gh release create`.
#
# Why this is a script and not inline YAML:
#
#   * `gh release create` takes its assets as POSITIONAL arguments
#     (`gh release create <tag> <file>...`). It has no `-a`/`--asset` flag;
#     passing one makes gh abort with "unknown shorthand flag: 'a' in -a".
#     The asset list is therefore assembled as a bash array and expanded
#     with "${assets[@]}" — never word-split, never glob-expanded.
#
#   * The build job uploads a *tree*. upload-artifact stores the least
#     common ancestor of the uploaded paths as the archive root, so
#     `dist/native/...`, `dist/source/...` and `dist/sbom/...` land under
#     `native/`, `source/` and `sbom/` inside the artifact zip. The
#     aggregate validation step and `sha256sum -c` both need one flat
#     directory, so the tree is flattened once, here, before either runs.
#
# Subcommands:
#
#   stage <src-dir> <flat-dir> <list-file>
#       Flatten <src-dir> into <flat-dir> — one file per release asset,
#       keyed by basename — and write the NUL-delimited list of staged
#       asset paths to <list-file>. The list file is what `create`
#       consumes; it is the single source of truth for the asset set.
#
#   create <tag> <list-file> <notes-file> [mode]
#       Call `gh release create <tag> --title ... --draft --notes-file
#       <notes-file> <assets...>` with the staged assets as positional
#       arguments. `mode` is `production` (the default) or `test`:
#
#         production — the title is the bare version, the release is an
#         ordinary draft, and the notes must be the tracked release
#         notes under `docs/releases/`; a release whose description is a
#         checksum dump is not a release.
#
#         test — a rehearsal of the release path. The title is prefixed
#         `TEST `, the release is a prerelease, and the notes carry a
#         banner that says it must not be published. A test release may
#         fall back to the checksum list as its description, because its
#         description is never the published one.
#
# Fail-closed by design:
#   * an empty source tree aborts before anything is staged;
#   * a destination directory that already holds files aborts, so stale
#     files can never be attached to a release;
#   * an empty asset list aborts before `gh` is called, so a release is
#     never created with zero assets;
#   * a missing notes file aborts, and in production mode the notes file
#     must be a tracked `docs/releases/` note;
#   * an unknown mode aborts;
#   * a duplicate basename is skipped with a warning, because a release
#     asset is named by its basename and two files with one name cannot
#     both be attached. RELEASE_SHA256.txt and MANIFEST.sha256 are not
#     special-cased: they are ordinary release assets and are attached
#     like any other file.

set -euo pipefail

# Byte-order collation for the sort below, so the staged order does not
# depend on the runner's locale. Deliberately not exported: `gh` keeps the
# runner's locale for its own output.
LC_ALL=C

if [ -z "${BASH_VERSION:-}" ]; then
  echo "::error::release-assets.sh requires bash" >&2
  exit 1
fi

die() {
  echo "::error::$1" >&2
  exit 1
}

warn() {
  echo "::warning::$1" >&2
}

# Flatten the artifact tree into a flat directory and emit the asset list.
cmd_stage() {
  if [ "$#" -ne 3 ]; then
    die "usage: release-assets.sh stage <src-dir> <flat-dir> <list-file>"
  fi
  local src=$1
  local flat=$2
  local list=$3

  [ -d "$src" ] || die "artifact tree not found: $src"
  flat=${flat%/}
  [ -n "$flat" ] || die "destination directory must not be empty"
  [ -n "$list" ] || die "asset list path must not be empty"

  if [ -e "$flat" ]; then
    [ -d "$flat" ] || die "destination exists and is not a directory: $flat"
    shopt -s nullglob dotglob
    local -a existing=("$flat"/*)
    shopt -u nullglob dotglob
    if [ "${#existing[@]}" -gt 0 ]; then
      die "destination directory is not empty: $flat (stale files would be attached to the release)"
    fi
  fi
  mkdir -p -- "$flat"

  # Collect every regular file under the tree. NUL-delimited, so names
  # containing spaces (or anything else odd) survive intact.
  local -a paths=()
  local f
  while IFS= read -r -d '' f; do
    paths+=("$f")
  done < <(find "$src" -type f -print0)

  if [ "${#paths[@]}" -eq 0 ]; then
    die "no release artifacts found under $src — refusing to stage an empty release"
  fi

  # Sort by path so the staged order (and therefore the asset order and
  # the warning about a skipped duplicate) is deterministic.
  local i j tmp
  for ((i = 1; i < ${#paths[@]}; i++)); do
    tmp=${paths[i]}
    j=$((i - 1))
    while [ "$j" -ge 0 ] && [ "${paths[j]}" \> "$tmp" ]; do
      paths[j+1]="${paths[j]}"
      j=$((j - 1))
    done
    paths[j+1]="$tmp"
  done

  local -a staged=()
  local base
  # The staged file is named by the source basename, so two sources with
  # the same basename collide. Comparing against the *previous* basename
  # only caught adjacent duplicates: a third file sorting between them
  # reset the comparison and the later duplicate silently overwrote the
  # earlier one, shipping the wrong bytes under that name. Test the
  # destination itself — that is the actual collision, adjacent or not.
  for f in "${paths[@]}"; do
    base=${f##*/}
    if [ -e "$flat/$base" ]; then
      warn "duplicate release asset name '$base' — skipping $f (a release asset is named by its basename)"
      continue
    fi
    cp -- "$f" "$flat/$base"
    staged+=("$flat/$base")
  done

  : > "$list"
  local p
  for p in "${staged[@]}"; do
    printf '%s\0' "$p" >> "$list"
  done

  echo "staged ${#staged[@]} release asset(s) from $src into $flat"
}

# Create the draft release with the staged assets as positional arguments.
cmd_create() {
  if [ "$#" -lt 3 ] || [ "$#" -gt 4 ]; then
    die "usage: release-assets.sh create <tag> <list-file> <notes-file> [production|test]"
  fi
  local tag=$1
  local list=$2
  local notes=$3
  local mode=${4:-production}

  case "$mode" in
    production|test) ;;
    *) die "unknown release mode '$mode' — expected 'production' or 'test'" ;;
  esac

  [ -f "$list" ] || die "asset list not found: $list"
  [ -f "$notes" ] || die "release notes file not found: $notes"

  # A production release carries its tracked release notes. The checksum
  # list is a manifest of the artifacts, not a description of the
  # release: a draft whose description is a SHA dump is a release nobody
  # can read, so the fallback is allowed only for a rehearsal.
  if [ "$mode" = production ]; then
    case "$notes" in
      docs/releases/*) ;;
      *) die "a production release must carry its tracked release notes (docs/releases/<tag>.md); got: $notes" ;;
    esac
  fi

  local -a assets=()
  local entry
  while IFS= read -r -d '' entry; do
    [ -n "$entry" ] || continue
    assets+=("$entry")
  done < "$list"

  if [ "${#assets[@]}" -eq 0 ]; then
    die "no release assets to attach — refusing to create a release with zero assets"
  fi

  for entry in "${assets[@]}"; do
    [ -f "$entry" ] || die "staged asset is missing: $entry"
  done

  # Assets are the positional filename arguments after the tag. `gh
  # release create` has no -a/--asset flag. The title is the bare
  # version ("0.2.6") — the repository name and the asset filenames
  # already carry the product name. A rehearsal is titled `TEST <version>`
  # and is a prerelease, so it cannot be mistaken for the release.
  local title="${tag#v}"
  local -a flags=()
  local notes_arg=$notes
  local banner=""
  if [ "$mode" = test ]; then
    title="TEST $title"
    flags=(--prerelease)
    banner=$(mktemp)
    {
      echo "**TEST RELEASE — not for publication.**"
      echo
      echo "This is a rehearsal of the release path. It is a prerelease, its"
      echo "title carries the TEST prefix, and it must not be published."
      echo
      cat "$notes"
    } > "$banner"
    notes_arg=$banner
    # shellcheck disable=SC2064
    trap "rm -f -- '$banner'" EXIT
  fi

  gh release create "$tag" \
    --title "$title" \
    --draft \
    ${flags[@]+"${flags[@]}"} \
    --notes-file "$notes_arg" \
    "${assets[@]}"
}

main() {
  local cmd=${1:-}
  case "$cmd" in
    stage)
      shift
      cmd_stage "$@"
      ;;
    create)
      shift
      cmd_create "$@"
      ;;
    *)
      die "usage: release-assets.sh {stage <src-dir> <flat-dir> <list-file> | create <tag> <list-file> <notes-file> [production|test]}"
      ;;
  esac
}

main "$@"
