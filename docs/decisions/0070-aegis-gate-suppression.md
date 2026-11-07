# 0070 — The Aegis gate suppresses rule classes and content digests, never line numbers

- **Date:** 2026-11-06
- **Status:** Accepted
- **Supersedes:** the `.aegis-baseline.json` mechanism described in
  `docs/SESSIONS.md` (2026-11-05) and referenced from `docs/NEXT.md`
- **Relates to:** `xtask/src/main.rs` (the `aegis` gate), `xtask/src/util.rs`,
  `.aegis-pattern-state.json`, `.aegis-allowlist.json`, `.aegisignore`,
  `docs/NEXT.md`, `docs/HANDOFF.md`

## Context

On 2026-11-06 the Aegis gate failed on six findings that had been
recorded in the committed baseline the day before. Nothing had leaked:
the 22.2 receipt prose quotes the *names* of the rules it describes
(`pci-cardholder-data`, `sql-query`, `ssrf-localhost`), so the receipt
trips the rules it is about.

The failure was not a false alarm in the finding sense — it was a
false alarm in the *keying* sense. Aegis's `--baseline` keys a finding
as `pattern:file:line:content-digest`, so the suppression is valid only
while the finding stays on exactly that line of exactly that file. Any
edit above it invalidates the entry. The fix at the time was to
regenerate the whole 19.4 MB baseline, which takes a full-tree scan
(2–15 minutes on this host, depending on load). A gate whose routine
response to an unrelated edit is a fifteen-minute regeneration is a
gate that gets skipped, and a skipped gate catches nothing.

The deeper problem is that the baseline was carrying 644 entries, of
which 634 were noise from a dozen rule classes that cannot fire in this
tree — every 5-digit literal read as a zip code (523), the word "pan"
in "pan-zoom" read as cardholder data (52), every Rust `.eval(t)` call
read as a code-eval risk (15). Only ten were genuine per-finding false
positives.

This decision also records a scanner comparison, because the choice of
scanner determines how much suppression a tree like this needs.

## The scanner comparison

Two scanners were run against the same corpus on the same box, three
interleaved runs each (`crates/`, aegis 0.6.3 production profile
against Atheon-Enhanced, the Go binary at
`/nas/Temp/atheon-bundle-fix-bin`):

| scanner | wall clock (3 runs) | median | findings |
|---|---|---|---|
| Aegis 0.6.3 (Rust) | 12.85 s / 12.12 s / 11.37 s | **12.12 s** | **373** |
| Atheon-Enhanced (Go) | 178.7 s / 216.3 s / 239.3 s | **216.3 s** | **18,021** |

Atheon-Enhanced is **17.8× slower** and produces **48× more findings**
on the same tree. Its rules also mis-attribute: terraform, azure and
dockerfile rules fire on Rust lines, while Aegis's labels were precise
on every class inspected. On a planted corpus both missed `ghp_` and
`xoxb_` tokens; Atheon-Enhanced caught a Stripe key Aegis missed, which
is not enough to outweigh 48× the noise. Aegis stays the scanner.

The comparison also produced the mechanism this decision turns on.
Aegis has two suppression channels, and only one of them works:

* `--baseline <file>` — keys on `pattern:file:line:content-digest`.
  Line-keyed, and therefore brittle. Measured: a finding moved down
  one line by an unrelated edit, and the baseline no longer covered it.
* `$HOME/.config/aegis/pattern-state.json` — `aegis disable <pattern>`
  writes `{"version":1,"disabled_patterns":[...]}` there. Keyed on the
  rule *name*, so it survives every edit.

A JSON profile passed with `-c <file>.json` looks like a third channel,
but its `disabled_patterns` field is silently ignored (measured: 7
`eval-usage` findings with and without it). The state file is the only
working per-rule switch.

## Decision

1. **Aegis 0.6.3 with the production profile stays the scanner**, on the
   measured evidence above.

2. **Rule classes are suppressed by name**, in a committed
   `.aegis-pattern-state.json`. The gate installs that file into a
   private `HOME` (`$TMPDIR/firecraft-aegis-home`) and runs the scanner
   with that `HOME`, so the developer's own Aegis configuration is never
   touched and CI gets the same suppression as a laptop.

3. **Individual findings are suppressed by content digest**, in a
   committed `.aegis-allowlist.json`. The key is
   `pattern:file:content-digest` and deliberately omits the line
   number: the digest is stable when the finding moves, and changes
   only when the matched content itself changes — which is exactly when
   a human should look again.

4. **The line-keyed baseline is removed.** `.aegis-baseline.json` was
   19.4 MB of generated JSON whose entries the gate could not keep
   alive. It is deleted from the repository, and `.aegisignore` now
   excludes the two suppression files instead (their own contents —
   rule names, digests, explanatory prose — land back in the findings,
   exactly as the baseline's did).

5. **The gate never writes the allowlist.** Suppressing a finding is a
   human decision that requires seeing the finding first, so
   `cargo run -p xtask -- aegis --report` prints the uncovered findings
   together with the key an allowlist entry would need, and the gate
   itself only reads.

6. **Nothing is suppressed before it is verified.** Every class name
   and every allowlist entry carries its evidence below or in the
   allowlist file's own `note` field.

## The suppressed rule classes, and why each cannot fire here

Counts are from a whole-tree scan of the pre-suppression tree (644
findings).

| rule | hits | why it cannot be a true positive here |
|---|---|---|
| `zip-code` | 523 | Matches any 5-digit run: CS6 command ids (`43404`), the JPEG dimension ceiling (`65535`), menu ids. This repository holds no postal data. |
| `pci-cardholder-data` | 52 | Matches the word "pan" — "pan-zoom", "hand pan", `pan` identifiers. Cardholder-number detection stays covered by the five `credit-card-*` rules, which remain **armed**. |
| `phone-number` | 21 | Matches any 10-digit run: `4294967295` (u32::MAX), CRC and chunk values, observed value ranges in the PNG node dictionary. |
| `eval-usage` | 15 | Every hit is a Rust `.eval(t)` call on a curve segment (`crates/vector/src/boolean.rs`, `offset.rs`). |
| `ssrf-localhost` | 8 | The workspace has no HTTP client — no `reqwest`, `ureq`, `isahc`, `curl` or `attohttpc` dependency — so there is no request a loopback URL could redirect. |
| `sql-query` | 7 | Matches English prose ("Select a tool from ..."). This repository has no database. |
| `path-traversal` | 3 | The rule's regex is JavaScript-shaped (`readFile\|readFileSync\|open\|fs\.` … `+` … `user\|input\|param\|query`) and fires on English prose ("openDocument … + … input") and on Rust `std::fs` idioms. |
| `security-hardening-path-traversal` | 1 | Same shape, same cause. |
| `international-phone` | 1 | Matches build metadata in a `Cargo.lock` version string (`0.6.0+11769913`). |
| `australian-tfn`, `bank-routing-number`, `ssn-no-dashes` | 3 | Match a 9-digit observed value range in the PNG node dictionary. |

Ten findings remain after the class suppression, and all ten are
per-finding false positives; they are the eight allowlist entries
below.

| finding | why it is a false positive |
|---|---|
| `bearer-token-url` in `docs/NEXT.md` | The GitForge row names the *file* that holds the credential. The row contains no token. |
| `credit-card-amex` and `credit-card-number-generic` in `crates/text/tests/layout.rs` | A glyph-position golden value whose long digit run matches the card-number shape. The literal itself is in the allowlist entry, not here — a record that quotes it would trip the rule it is describing. |
| `xml-external-entity` and `security-hardening-xml-external-entity` in `crates/io/tests/svgimport.rs` | The "entity expansion" fixture, which asserts that the SVG importer *refuses* an external entity — so the payload must declare one. |
| `xml-external-entity` and `security-hardening-xml-external-entity` in `packaging/build-macos-dmg.sh` | The Apple plist doctype. The rule matches the doctype's quoted public identifier; the plist declares no entity. |
| `stack-trace-exposure` in `infrastructure/docker/ci-rust.Dockerfile` | A comment quoting a V8 crash dump observed in CI. It documents a failure; the build prints nothing. |

## Consequences

- The gate is fast again and, more importantly, it is *stable*: an
  ordinary edit above a finding no longer invalidates its suppression.
- The suppression shrank from 644 line-keyed entries in 19.4 MB to 12
  rule names and 8 content digests.
- Detection did not shrink with the noise. The credential rules —
  private keys, AWS keys, GitHub/Slack/Stripe/Twilio tokens, `jwt`,
  `bearer-token`, `password-in-url`, command injection — were never
  suppressed and remain armed, and the five `credit-card-*` rules
  remain armed so that a real card number is still caught.
- One blind spot is accepted deliberately: the digest is of the matched
  text, so the *same* matched text elsewhere in the *same* file is also
  covered. In `crates/io/tests/svgimport.rs` that is the entity
  declaration of the deliberate XXE fixture, which is the only place
  the file names one.
- `docs/NEXT.md`'s "do not add baseline entries without verifying each
  finding is a false positive" now reads as a rule about the allowlist.
  It is unchanged in force: an entry is a claim that a human checked.
