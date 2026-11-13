#!/usr/bin/env bash
# Self-test for packaging/release-assets.sh — the script that stages the
# downloaded release artifacts and builds the `gh release create` call.
#
# The release path is the campaign's critical path and it has no unit
# tests: `gh` is called once, at the end of a tag run, on a runner. A
# regression in the argument vector — a missing asset, a title that no
# longer matches the version, a rehearsal that looks like a release —
# would only surface when a release is being cut.
#
# So the script is exercised here against a stub `gh` that records its
# argument vector instead of calling GitHub, and every failure mode is
# demonstrated with one deliberately broken input.
#
# Usage: bash tests/release-assets-selftest.sh
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
script="$root/packaging/release-assets.sh"

work="$(mktemp -d)"
trap 'find "$work" -type f -delete; find "$work" -type d -empty -delete' EXIT

failures=0
passed=0

ok() {
  printf 'ok   %s\n' "$1"
  passed=$((passed + 1))
}

bad() {
  printf 'FAIL %s\n' "$1" >&2
  shift
  for line in "$@"; do
    printf '     %s\n' "$line" >&2
  done
  failures=$((failures + 1))
}

# A stub `gh` that records the argument vector and, when it is handed a
# --notes-file, copies that file's contents beside the record — so the
# banner a rehearsal carries can be inspected after the caller exits.
make_stub() {
  local bin="$1"
  mkdir -p "$bin"
  cat > "$bin/gh" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "$@" > "$GH_ARGV_FILE"
notes=""
prev=""
for arg in "$@"; do
  if [ "$prev" = "--notes-file" ]; then
    notes=$arg
  fi
  prev=$arg
done
if [ -n "$notes" ] && [ -f "$notes" ]; then
  cp -- "$notes" "$GH_NOTES_CAPTURE"
fi
STUB
  chmod +x "$bin/gh"
}

# Build a small artifact tree and its notes.
make_tree() {
  local dir="$1"
  mkdir -p "$dir/tree/native" "$dir/tree/source" "$dir/tree/sbom" "$dir/docs/releases"
  printf 'desktop\n' > "$dir/tree/native/firecraft-0.2.6-linux-x86_64.tar.gz"
  printf 'cli\n' > "$dir/tree/native/firecraft-cli-0.2.6-linux-x86_64.tar.gz"
  printf 'deb\n' > "$dir/tree/native/firecraft_0.2.6_amd64.deb"
  printf 'source\n' > "$dir/tree/source/firecraft-0.2.6-source.tar.gz"
  printf 'sbom\n' > "$dir/tree/sbom/firecraft-0.2.6-sbom.cdx.json"
  printf 'checksums\n' > "$dir/tree/RELEASE_SHA256.txt"
  printf '# FireCraft 0.2.6\n\nRelease notes.\n' > "$dir/docs/releases/v0.2.6.md"
  printf 'checksums\n' > "$dir/RELEASE_SHA256.txt"
}

# --- stage ---------------------------------------------------------------
case_dir="$work/stage"
make_tree "$case_dir"
make_stub "$case_dir/bin"
out=$(cd "$case_dir" && PATH="$case_dir/bin:$PATH" bash "$script" stage tree flat assets.zlist 2>&1)
if [ $? -ne 0 ]; then
  bad "stage flattens the artifact tree" "$out"
else
  staged=$(find "$case_dir/flat" -type f | wc -l)
  listed=$(tr -cd '\0' < "$case_dir/assets.zlist" | wc -c)
  if [ "$staged" -eq 6 ] && [ "$listed" -eq 6 ]; then
    ok "stage flattens the artifact tree (6 assets, 6 listed)"
  else
    bad "stage flattens the artifact tree" "staged=$staged listed=$listed"
  fi
fi

# --- stage rejects an empty tree ----------------------------------------
empty="$work/empty"
mkdir -p "$empty/tree"
out=$(cd "$empty" && bash "$script" stage tree flat assets.zlist 2>&1)
if [ $? -ne 0 ]; then
  ok "stage refuses an empty artifact tree"
else
  bad "stage refuses an empty artifact tree" "$out"
fi

# --- stage refuses a stale destination -----------------------------------
stale="$work/stale"
make_tree "$stale"
mkdir -p "$stale/flat"
printf 'left over\n' > "$stale/flat/old-artifact.tar.gz"
out=$(cd "$stale" && bash "$script" stage tree flat assets.zlist 2>&1)
if [ $? -ne 0 ]; then
  ok "stage refuses a destination that already holds files"
else
  bad "stage refuses a destination that already holds files" "$out"
fi

# --- stage skips a duplicate basename ------------------------------------
dup="$work/dup"
make_tree "$dup"
mkdir -p "$dup/tree/other"
printf 'second copy with the same name\n' > "$dup/tree/other/firecraft_0.2.6_amd64.deb"
out=$(cd "$dup" && bash "$script" stage tree flat assets.zlist 2>&1)
if [ $? -eq 0 ] && [ "$(find "$dup/flat" -type f -name 'firecraft_0.2.6_amd64.deb' | wc -l)" -eq 1 ] \
  && printf '%s' "$out" | grep -q 'duplicate release asset name'; then
  ok "stage skips a duplicate basename with a warning"
else
  bad "stage skips a duplicate basename with a warning" "$out"
fi

# --- create, production mode --------------------------------------------
prod="$work/prod"
make_tree "$prod"
make_stub "$prod/bin"
(cd "$prod" && PATH="$prod/bin:$PATH" bash "$script" stage tree flat assets.zlist >/dev/null 2>&1)
GH_ARGV_FILE="$prod/argv" GH_NOTES_CAPTURE="$prod/notes-capture" \
  out=$(cd "$prod" && PATH="$prod/bin:$PATH" GH_ARGV_FILE="$prod/argv" \
    GH_NOTES_CAPTURE="$prod/notes-capture" \
    bash "$script" create v0.2.6 assets.zlist docs/releases/v0.2.6.md production 2>&1)
argv=$(cat "$prod/argv" 2>/dev/null || true)
if printf '%s\n' "$argv" | grep -qx -- '--draft' \
  && printf '%s\n' "$argv" | grep -qx -- '0.2.6' \
  && ! printf '%s\n' "$argv" | grep -qx -- '--prerelease' \
  && ! printf '%s\n' "$argv" | grep -q '^TEST '; then
  ok "create in production mode is a draft titled by the version"
else
  bad "create in production mode is a draft titled by the version" "$argv"
fi
if [ "$(grep -c . "$prod/argv")" -ge 9 ]; then
  ok "create attaches every staged asset as a positional argument"
else
  bad "create attaches every staged asset as a positional argument" "$(cat "$prod/argv")"
fi

# --- create, test mode ---------------------------------------------------
testmode="$work/test"
make_tree "$testmode"
make_stub "$testmode/bin"
(cd "$testmode" && PATH="$testmode/bin:$PATH" bash "$script" stage tree flat assets.zlist >/dev/null 2>&1)
out=$(cd "$testmode" && PATH="$testmode/bin:$PATH" GH_ARGV_FILE="$testmode/argv" \
  GH_NOTES_CAPTURE="$testmode/notes-capture" \
  bash "$script" create v0.2.6-dryrun.1 assets.zlist RELEASE_SHA256.txt test 2>&1)
argv=$(cat "$testmode/argv" 2>/dev/null || true)
if printf '%s\n' "$argv" | grep -qx -- '--prerelease' \
  && printf '%s\n' "$argv" | grep -qx -- 'TEST 0.2.6-dryrun.1'; then
  ok "create in test mode is a prerelease titled TEST <version>"
else
  bad "create in test mode is a prerelease titled TEST <version>" "$argv"
fi
if head -1 "$testmode/notes-capture" 2>/dev/null | grep -q 'TEST RELEASE'; then
  ok "create in test mode banners the notes as not for publication"
else
  bad "create in test mode banners the notes as not for publication" "$(head -3 "$testmode/notes-capture" 2>/dev/null)"
fi

# --- production refuses notes that are not the tracked release notes -----
refuse="$work/refuse"
make_tree "$refuse"
make_stub "$refuse/bin"
(cd "$refuse" && PATH="$refuse/bin:$PATH" bash "$script" stage tree flat assets.zlist >/dev/null 2>&1)
out=$(cd "$refuse" && PATH="$refuse/bin:$PATH" GH_ARGV_FILE="$refuse/argv" \
  GH_NOTES_CAPTURE="$refuse/notes-capture" \
  bash "$script" create v0.2.6 assets.zlist RELEASE_SHA256.txt production 2>&1)
if [ $? -ne 0 ] && [ ! -f "$refuse/argv" ] \
  && printf '%s' "$out" | grep -q 'tracked release notes'; then
  ok "create in production mode refuses a checksum dump as the description"
else
  bad "create in production mode refuses a checksum dump as the description" "$out"
fi

# --- create refuses a missing notes file ---------------------------------
missing="$work/missing"
make_tree "$missing"
make_stub "$missing/bin"
(cd "$missing" && PATH="$missing/bin:$PATH" bash "$script" stage tree flat assets.zlist >/dev/null 2>&1)
out=$(cd "$missing" && PATH="$missing/bin:$PATH" GH_ARGV_FILE="$missing/argv" \
  GH_NOTES_CAPTURE="$missing/notes-capture" \
  bash "$script" create v0.2.6 assets.zlist docs/releases/v0.9.9.md production 2>&1)
if [ $? -ne 0 ] && [ ! -f "$missing/argv" ]; then
  ok "create refuses a missing notes file"
else
  bad "create refuses a missing notes file" "$out"
fi

# --- create refuses an empty asset list ----------------------------------
emptyassets="$work/emptyassets"
make_tree "$emptyassets"
make_stub "$emptyassets/bin"
: > "$emptyassets/assets.zlist"
out=$(cd "$emptyassets" && PATH="$emptyassets/bin:$PATH" GH_ARGV_FILE="$emptyassets/argv" \
  GH_NOTES_CAPTURE="$emptyassets/notes-capture" \
  bash "$script" create v0.2.6 assets.zlist docs/releases/v0.2.6.md production 2>&1)
if [ $? -ne 0 ] && [ ! -f "$emptyassets/argv" ]; then
  ok "create refuses to attach zero assets"
else
  bad "create refuses to attach zero assets" "$out"
fi

# --- create refuses an unknown mode --------------------------------------
unknown="$work/unknown"
make_tree "$unknown"
make_stub "$unknown/bin"
(cd "$unknown" && PATH="$unknown/bin:$PATH" bash "$script" stage tree flat assets.zlist >/dev/null 2>&1)
out=$(cd "$unknown" && PATH="$unknown/bin:$PATH" GH_ARGV_FILE="$unknown/argv" \
  GH_NOTES_CAPTURE="$unknown/notes-capture" \
  bash "$script" create v0.2.6 assets.zlist docs/releases/v0.2.6.md rehearsal 2>&1)
if [ $? -ne 0 ] && [ ! -f "$unknown/argv" ]; then
  ok "create refuses an unknown release mode"
else
  bad "create refuses an unknown release mode" "$out"
fi

if [ "$failures" -ne 0 ]; then
  printf '%s of %s self-test case(s) failed\n' "$failures" "$((passed + failures))" >&2
  exit 1
fi
printf 'all %s self-test cases passed\n' "$passed"
