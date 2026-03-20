### S2c — Single-parser backend contract guards

- **User/system value**: makes the shared-normalizer handoff reviewable and hard to regress by
  proving neither backend mapping surface re-parses `agent_api.config.model.v1` from raw request
  extensions.
- **Scope (in/out)**:
  - In:
    - focused backend contract/source tests for the Codex and Claude mapping modules.
    - narrow assertions that `normalize.rs` owns the raw extension parse.
    - regression checks that backend mapping code consumes the normalized handoff.
  - Out:
    - capability advertising behavior.
    - argv placement, absence behavior, or fork/runtime rejection execution tests.
    - broad source snapshots or formatting-sensitive assertions.
- **Acceptance criteria**:
  - Codex and Claude contract guards fail if backend mapping code reintroduces direct reads of
    `request.extensions["agent_api.config.model.v1"]`.
  - The assertion remains narrow enough to avoid freezing unrelated source layout.
- **Dependencies**:
  - `MS-C09` from `SEAM-2`.
  - the single-parser rule in `threading.md` and `scope_brief.md`.
- **Verification**:
  - `cargo test -p agent_api codex`
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Use the existing `include_str!` contract-test style already present in backend suites.

#### S2c.T1 — Shared-normalizer conformance guard against backend-local re-parsing

- **Outcome**: the backend contract tests pin the single-parser rule without coupling to unrelated
  code shape.
- **Files**:
  - `crates/agent_api/src/backends/codex/tests/backend_contract.rs`
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`

Checklist:
- Implement:
  - Add single-parser regression assertions in the backend contract test modules.
  - Keep the assertion focused on raw-extension access for `agent_api.config.model.v1`.
- Test:
  - `cargo test -p agent_api codex`
  - `cargo test -p agent_api claude_code`
- Validate:
  - Allow `normalize.rs` to own the raw key.
  - Disallow backend-local reads in mapping modules.
