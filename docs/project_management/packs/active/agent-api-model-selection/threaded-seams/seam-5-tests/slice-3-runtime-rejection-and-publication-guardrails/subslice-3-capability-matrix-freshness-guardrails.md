### S3c — Capability-matrix freshness guardrails

- **User/system value**: makes stale capability publication merge-blocking when
  `agent_api.config.model.v1` advertising changes.
- **Scope (in/out)**:
  - In:
    - generator-driven freshness validation for
      `docs/specs/universal-agent-api/capability-matrix.md`
    - pre-advertising versus post-advertising expectations tied to actual backend capability state
    - wiring the freshness check into the validation path used for this feature
  - Out:
    - backend-specific runtime-rejection fixtures or stream assertions
    - hand-maintained matrix parsers or duplicated capability tables
- **Acceptance criteria**:
  - A stale committed `capability-matrix.md` fails validation once advertising changes land.
  - The check distinguishes pre-advertising and post-advertising states correctly.
  - The generator remains the source of truth for the committed artifact.
- **Dependencies**:
  - `MS-C05` and `MS-C08` from `SEAM-2`
- **Verification**:
  - `cargo run -p xtask -- capability-matrix`
  - `make test`
- **Rollout/safety**:
  - Land this with the same change that flips advertising so publication and capability exposure
    cannot drift.

#### S3.T3 — Capability-matrix freshness assertion for model-selection advertising

- **Outcome**: capability publication is validated by regeneration, not by hand-maintained textual
  expectations.
- **Files**:
  - `docs/specs/universal-agent-api/capability-matrix.md`
  - `crates/xtask/src/**`
  - existing validation hooks used by this feature's test path

Checklist:
- Implement:
  - Add or document a freshness check that reruns `cargo run -p xtask -- capability-matrix`.
  - Tie the expectation to actual advertising state rather than assuming a permanent row.
  - Wire the freshness check into the validation path already used for this feature.
- Test:
  - `cargo run -p xtask -- capability-matrix`
  - `make test`
- Validate:
  - Confirm stale matrix diffs block the change once advertising lands.
  - Avoid introducing a second parser for the matrix artifact.
