### S1 — Exec/resume model handoff and argv mapping

- **User/system value**: gives Codex exec/resume flows the minimal end-to-end v1 behavior quickly: accepted model-selection requests reach the existing builder/argv surface as exactly one `--model <trimmed-id>`, and absent requests preserve current defaults.
- **Scope (in/out)**:
  - In:
    - consume the SEAM-2 helper output instead of reading `request.extensions["agent_api.config.model.v1"]` in Codex-specific code
    - add a Codex policy field for the effective trimmed model id
    - thread that value through backend/harness spawn paths into `exec.rs`
    - call `CodexClientBuilder::model(trimmed_model_id)` for exec and resume flows only
    - prove argv placement matches the Codex streaming exec contract: wrapper-owned overrides first, then `--model <trimmed-id>`, then any accepted `--add-dir` entries
  - Out:
    - fork-flow rejection behavior (S2)
    - runtime model rejection translation after the stream has opened (S3)
    - capability advertising and matrix publication (SEAM-2)
- **Acceptance criteria**:
  - SEAM-3 consumes only SEAM-2's typed helper result for this key; no new raw parse sites appear outside `crates/agent_api/src/backend_harness/normalize.rs`.
  - When `agent_api.config.model.v1` is absent, Codex exec/resume omit `.model(...)` and emit no `--model`.
  - When the key is present and valid, Codex exec/resume emit exactly one `--model <trimmed-id>` through the existing builder path.
  - Argv ordering matches `docs/specs/codex-streaming-exec-contract.md` for both exec and resume flows.
- **Dependencies**:
  - `SEAM-2` / `MS-C09` shared model-normalizer handoff
  - `SEAM-1` / `MS-C02` absence semantics
  - `MS-C06` Codex mapping contract
- **Verification**:
  - targeted Codex backend/unit tests for policy extraction and argv layout
  - regression check that no second parser for `agent_api.config.model.v1` exists outside `normalize.rs`
- **Rollout/safety**:
  - additive and deterministic: only requests carrying the new key change behavior
  - safest first slice because it leaves fork/runtime-rejection handling unchanged until later slices pin them

#### S1.T1 — Adopt SEAM-2's normalized helper output in Codex policy extraction

- **Outcome**: Codex policy extraction carries `Option<String>` for the effective model id without re-reading raw extension payloads.
- **Inputs/outputs**:
  - Input: `MS-C09` helper from `crates/agent_api/src/backend_harness/normalize.rs`
  - Output: updates in `crates/agent_api/src/backends/codex/policy.rs`, `crates/agent_api/src/backends/codex/harness.rs`, and any shared policy types so Codex receives `model: Option<String>` alongside existing exec/fork selectors
- **Implementation notes**:
  - keep ownership of trimming, bounds checks, and `InvalidRequest { message: "invalid agent_api.config.model.v1" }` in SEAM-2
  - preserve current session resume/fork mutual-exclusion validation
  - treat `None` as the only legal "no override" representation
- **Acceptance criteria**:
  - Codex code compiles against the shared helper output
  - no Codex module parses or trims `agent_api.config.model.v1` directly
- **Test notes**:
  - add or extend policy/harness tests to show helper output reaches the Codex policy unchanged
  - validate with `rg -n "agent_api\\.config\\.model\\.v1" crates/agent_api/src/backends/codex crates/agent_api/src/backend_harness/normalize.rs`
- **Risk/rollback notes**:
  - low risk if kept as a typed-policy handoff only; rollback is localized to policy plumbing

Checklist:
- Implement: add `model: Option<String>` to the Codex policy path and source it from the shared helper output.
- Test: run targeted Codex backend/policy tests.
- Validate: confirm `normalize.rs` remains the only raw parse site for `agent_api.config.model.v1`.
- Cleanup: remove any temporary duplicate plumbing or dead helper code.

#### S1.T2 — Map exec/resume flows to `CodexClientBuilder::model(...)`

- **Outcome**: accepted model-selection requests change only exec/resume builder construction and surface as exactly one `--model <trimmed-id>`.
- **Inputs/outputs**:
  - Input: `model: Option<String>` from `S1.T1`
  - Output: updates in `crates/agent_api/src/backends/codex/backend.rs`, `crates/agent_api/src/backends/codex/exec.rs`, and supporting tests under `crates/agent_api/src/backends/codex/tests/`
- **Implementation notes**:
  - add the model field to `ExecFlowRequest`
  - call `builder.model(trimmed_model_id)` only when `Some(...)`
  - rely on `crates/codex/src/builder/mod.rs` for final `--model` emission rather than hand-writing argv fragments
  - keep placement consistent with wrapper-owned overrides and any later `--add-dir` emission
- **Acceptance criteria**:
  - exec/resume emit one `--model` pair when present, zero when absent
  - argv ordering matches `docs/specs/codex-streaming-exec-contract.md`
  - no unrelated Codex policy or sandbox semantics change
- **Test notes**:
  - add builder/argv layout assertions in `crates/agent_api/src/backends/codex/tests/mapping.rs` or the closest existing test module
  - include both exec and resume coverage
- **Risk/rollback notes**:
  - medium risk because it touches run wiring, but scope is bounded to exec/resume only

Checklist:
- Implement: thread `model: Option<String>` into `ExecFlowRequest` and apply `.model(...)` in `exec.rs`.
- Test: add exec/resume argv tests covering present and absent model selection.
- Validate: inspect the final argv path rather than asserting on intermediate policy state alone.
- Cleanup: avoid any manual `--model` string assembly outside the builder.
