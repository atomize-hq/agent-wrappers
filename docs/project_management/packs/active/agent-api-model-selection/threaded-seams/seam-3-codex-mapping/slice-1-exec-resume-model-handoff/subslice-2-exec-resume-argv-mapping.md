### S1b — Exec/resume argv mapping through the existing builder path

- **User/system value**: lands the user-visible Codex behavior once the typed policy handoff exists, making exec and resume emit exactly one `--model <trimmed-id>` while leaving fork/runtime error paths to later slices.
- **Scope (in/out)**:
  - In:
    - thread `model: Option<String>` from the Codex request path into exec/resume flow construction
    - call `CodexClientBuilder::model(trimmed_model_id)` only for `Some(...)`
    - pin final argv ordering for exec and resume against the streaming exec contract
  - Out:
    - new parsing or normalization logic for the model key
    - fork-flow rejection behavior
    - runtime rejection/event translation after stream open
- **Acceptance criteria**:
  - exec/resume emit exactly one `--model <trimmed-id>` pair when the typed model is present
  - exec/resume emit no `--model` when the typed model is absent
  - argv ordering matches `docs/specs/codex-streaming-exec-contract.md`
  - no manual `--model` string assembly appears outside the existing builder surface
- **Dependencies**:
  - `S1a` for `model: Option<String>` plumbing
  - `SEAM-1` / `MS-C02` absence semantics
  - `MS-C06` Codex mapping contract
  - existing `crates/codex` builder support for `.model(...)`
- **Verification**:
  - targeted exec/resume mapping tests under `crates/agent_api/src/backends/codex/tests/`
  - inspect final argv emission rather than intermediate policy state alone
- **Rollout/safety**:
  - bounded to exec/resume only; fork behavior and runtime rejection remain isolated in `S2` and `S3`

#### S1b.T1 — Map exec/resume flows to `CodexClientBuilder::model(...)`

- **Outcome**: accepted model-selection requests change only exec/resume builder construction and surface as exactly one `--model <trimmed-id>`.
- **Files**:
  - `crates/agent_api/src/backends/codex/backend.rs`
  - `crates/agent_api/src/backends/codex/exec.rs`
  - `crates/agent_api/src/backends/codex/tests/mapping.rs`
  - `crates/agent_api/src/backends/codex/tests/backend_contract.rs`

Checklist:
- Implement:
  - thread `model: Option<String>` into `ExecFlowRequest`
  - call `.model(trimmed_model_id)` only when `Some(...)`
  - rely on the existing builder path for final `--model` emission and placement
- Test:
  - add exec and resume coverage for present and absent model selection
  - assert exactly-one `--model` pair and the expected ordering relative to wrapper-owned overrides and `--add-dir`
- Validate:
  - confirm the current `crates/codex` builder surface remains sufficient and does not require a second packet
  - verify no unrelated Codex sandbox or policy semantics change
