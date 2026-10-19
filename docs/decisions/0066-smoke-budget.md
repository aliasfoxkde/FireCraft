# Decision 0066 — Smoke budget

## Status: Policy

The bridge smoke test uses `--virtual-time-budget=60000` (60 seconds of
virtual time) as a single ceiling covering bundle load + wasm init + the
four-message round trip through the host bridge.

## Question

Should wasm load time be budgeted separately from the application round
trip? A 60-second budget could mask a slow-loading bundle if the
application itself is fast.

## Analysis

`--virtual-time-budget` in headless Chromium advances virtual time only
while JavaScript executes. Network requests (wasm fetch) and wasm
compilation do **not** consume virtual time — they block the browser
event loop and use wall clock.

Therefore, the 60-second budget covers only:
1. JS execution (trivial)
2. Wasm init (sub-second once fetched)
3. `openDocument` message + reply (document deserialization)
4. `apply object.createPath` + reply
5. `exportPng` + reply
6. `exportDocument` + reply

Bundle fetch and wasm compilation are unbounded by virtual time — they
add wall-clock latency but do not affect the budget. This means the
current 60-second budget is generous for the application logic it covers.

## Decision

**No change to the budget.** The `--virtual-time-budget=60000` ceiling is
appropriate. It covers the application round trip without being vulnerable
to slow wasm fetch/compile times on the runner.

If wasm fetch is a concern (large bundle), the ceiling to tighten is the
14 MB wasm size budget enforced by `enforce_wasm_budget`, not the
virtual time budget.

## References

- `xtask/src/dist.rs`: `smoke_bridge` — `chromium_path` + `--virtual-time-budget=60000`
- `WASM_SIZE_CEILING: u64 = 14_000_000` — the real wasm load budget
