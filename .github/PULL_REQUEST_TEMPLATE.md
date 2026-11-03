<!-- Describe the behavior change and how you verified it. -->

## Summary

<!-- What and why, in a few sentences. -->

## Verification

- [ ] `cargo run -p xtask -- ci` passes (fmt, clippy `-D warnings`,
      tests, layer enforcement, wasm build)
- [ ] New behavior is covered by tests
- [ ] Any new non-code asset has a `docs/ATTRIBUTION.md` row
- [ ] User-visible changes noted in `docs/CHANGELOG.md`

## Clean-room attestation

This change contains no code, assets, or extracted constants copied
from proprietary sources; anything derived from observed behavior is
written fresh against public specs and evidence.
