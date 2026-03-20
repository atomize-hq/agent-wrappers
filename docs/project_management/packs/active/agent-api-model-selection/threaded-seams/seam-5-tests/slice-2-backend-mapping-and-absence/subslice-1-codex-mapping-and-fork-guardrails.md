### S2a — Codex mapping, absence behavior, and fork guardrails

- **User/system value**: proves the Codex backend consumes one trimmed normalized value, advertises
  support only when deterministic, preserves default behavior when the key is absent, and rejects
  unsupported fork handling before any app-server work begins.
- **Scope (in/out)**:
  - In:
    - Codex capability advertising assertions for `agent_api.config.model.v1`.
    - exec/resume mapping with exactly one `--model <trimmed-id>` pair.
    - absence/default behavior with no emitted `--model`.
    - pre-handle fork rejection coverage for accepted model-selection inputs.
  - Out:
    - Claude argv placement and `--fallback-model` exclusions.
    - shared single-parser source guards beyond any Codex-local references needed for this task.
    - post-stream runtime rejection and terminal `Error` event behavior in `S3`.
- **Acceptance criteria**:
  - Codex capability tests pin `agent_api.config.model.v1` advertising only in the deterministic
    support state.
  - Exec/resume flows emit exactly one `--model <trimmed-id>` pair; absence omits it.
  - Fork paths reject accepted values before `thread/list`, `thread/fork`, or `turn/start`.
  - Tests fail if raw whitespace-padded values leak into argv.
- **Dependencies**:
  - `MS-C05` and `MS-C09` from `SEAM-2`.
  - `MS-C06` from `SEAM-3`.
  - `docs/specs/codex-streaming-exec-contract.md`.
  - `docs/specs/codex-app-server-jsonrpc-contract.md`.
- **Verification**:
  - `cargo test -p agent_api codex`
- **Rollout/safety**:
  - Tests only. Keep all assertions inside existing Codex test modules.

#### S2a.T1 — Codex capability, exec/resume mapping, and fork-rejection tests

- **Outcome**: Codex model-selection coverage pins capability advertising, trimmed mapping,
  absence/default behavior, and the safe pre-handle fork rejection path.
- **Files**:
  - `crates/agent_api/src/backends/codex/tests/capabilities.rs`
  - `crates/agent_api/src/backends/codex/tests/mapping.rs`
  - `crates/agent_api/src/backends/codex/tests/app_server.rs`

Checklist:
- Implement:
  - Add capability assertions for `agent_api.config.model.v1`.
  - Add exec/resume mapping tests for trimmed success and absence.
  - Add fork rejection tests for the pinned safe backend message.
- Test:
  - `cargo test -p agent_api codex`
- Validate:
  - Confirm `--model` appears exactly once and only with the trimmed value.
  - Confirm fork rejection stays pre-handle and does not continue into app-server requests.
