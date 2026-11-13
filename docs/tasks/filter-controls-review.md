<!-- coderabbit-ai-generated:v1 -->

# Filter controls and master transform review follow-up

- Objective: preserve filter rename/bake edits across frames and retain the caller transform after the master-layer pass.
- Authority: maintainer review findings, verified against the current source and decisions 0113/0115.
- Scope: persistent Shell control state, transform restoration, focused regression tests.
<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:206a2a6611cf5061b412270ba014faa6ac9a034cae2b9405c72528ad2ddc1125 -->

- Status: fixed and validated. Five filter tests and five master-page render tests pass. App library: 273 passed, one markup-restyling failure reproduced against unchanged HEAD. Formatting and whitespace checks pass. No browser run, full CI, commit or push performed.

<!-- /coderabbit-ai-generated:v1 -->
- Completion: app library and master-page render tests pass, with formatting and whitespace checks clean.
