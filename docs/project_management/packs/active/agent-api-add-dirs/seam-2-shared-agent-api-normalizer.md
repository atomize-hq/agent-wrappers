# SEAM-2 — Shared `agent_api` add-dir normalizer

- **Name**: shared add-dir request parsing / validation / resolution helper
- **Type**: integration
- **Goal / user value**: compute the effective add-dir set once so Codex and Claude Code consume
  the same validated directory list and error posture.

## Scope

- In:
  - Introduce shared code that:
    - parses `agent_api.exec.add_dirs.v1`,
    - enforces the closed schema and bounds,
    - resolves relative paths from the effective working directory (per
      `docs/specs/universal-agent-api/contract.md` "Working directory resolution (effective working directory)"),
    - lexically normalizes,
    - validates `exists && is_dir`,
    - deduplicates while preserving order,
    - returns safe/testable errors without path leaks.
  - Define one shared normalizer entrypoint in
    `crates/agent_api/src/backend_harness/normalize.rs`:
    `normalize_add_dirs_v1(raw: Option<&serde_json::Value>, effective_working_dir: &Path) -> Result<Vec<PathBuf>, AgentWrapperError>`.
    The helper MUST return `Ok(Vec::new())` when
    `request.extensions.get("agent_api.exec.add_dirs.v1")` is `None`.
  - Require backend adapters to compute the effective working directory before invoking the shared
    helper. The concrete call sites are
    `crates/agent_api/src/backends/codex/harness.rs::CodexHarnessAdapter::validate_and_extract_policy(...)`
    and
    `crates/agent_api/src/backends/claude_code/harness.rs::ClaudeHarnessAdapter::validate_and_extract_policy(...)`.
    Those functions own working-directory precedence resolution; the helper itself MUST NOT read
    backend config defaults, request `working_dir`, or backend-internal fallbacks directly.
  - Export exactly `Vec<PathBuf>` as the normalized unique directory list consumed by backend
    policy extraction and spawn layers.
- Out:
  - Backend capability advertising.
  - Backend-specific CLI argv emission.

## Primary interfaces (contracts)

- **Normalizer input contract**
  - **Inputs**:
    - `request.extensions.get("agent_api.exec.add_dirs.v1")` as `Option<&serde_json::Value>`
    - the already-resolved effective working directory (per
      `docs/specs/universal-agent-api/contract.md` "Working directory resolution (effective
      working directory)")
  - **Outputs**:
    - `Ok(Vec::new())` when the key is absent
    - otherwise, the normalized unique directory list or `InvalidRequest`

- **Backend-consumption contract**
  - **Inputs**:
    - normalized directory list attached to the backend policy struct during
      `validate_and_extract_policy(...)`
  - **Outputs**:
    - backend policy/spawn layers receive `Vec<PathBuf>` and map it without re-validating
      schema/path semantics
    - downstream code MUST NOT reread the raw `AgentWrapperRunRequest.extensions` payload for this
      key

- **Shared helper entrypoint**
  - **Inputs**:
    - `request.extensions.get("agent_api.exec.add_dirs.v1")`
    - effective working directory selected for the run by the backend adapter layer before spawn
  - **Outputs**:
    - `Result<Vec<PathBuf>, AgentWrapperError>` from
      `backend_harness::normalize::normalize_add_dirs_v1(...)`

## Key invariants / rules

- The shared helper is the only place that decides trimming, path resolution, normalization, and
  dedup behavior.
- The shared helper MUST be invoked exactly once per run surface after capability gating and
  session-selector parsing, even when the add-dir key is absent.
- Backend config defaults and backend-internal working-directory fallbacks MUST be resolved before
  entering `normalize_add_dirs_v1(...)`; the helper MUST receive only the already-selected
  effective working directory.
- Pre-spawn filesystem validation for this feature (`exists && is_dir`) happens inside
  `normalize_add_dirs_v1(...)`; backend policy/spawn layers MUST NOT repeat it elsewhere.
- Backend-specific policy structs may carry the resulting `Vec<PathBuf>`, but they MUST treat it as
  an already-normalized value and MUST NOT duplicate schema or filesystem validation.
- Errors identify the failing field or index using the exact templates
  `invalid agent_api.exec.add_dirs.v1`, `invalid agent_api.exec.add_dirs.v1.dirs`, or
  `invalid agent_api.exec.add_dirs.v1.dirs[<i>]`, and never the raw path text.
- Backends MUST NOT invent any other `InvalidRequest` message shape for this key.
- The helper must not create a new working-directory precedence ladder that diverges from actual
  backend execution behavior.

## Dependencies

- Blocks: SEAM-3/4/5
- Blocked by: SEAM-1

## Touch surface

- `crates/agent_api/src/backend_harness/normalize.rs`
- `crates/agent_api/src/backend_harness/contract.rs`
- `crates/agent_api/src/backends/codex/policy.rs`
- `crates/agent_api/src/backends/claude_code/backend.rs`

## Verification

- Unit tests for:
  - non-object value rejection,
  - unknown key rejection,
  - missing `dirs`,
  - non-array `dirs`,
  - length bounds,
  - non-string entries,
  - trimmed empty entries,
  - byte-length bounds,
  - relative resolution,
  - lexical normalization,
  - dedup order preservation,
  - missing/non-directory path rejection,
  - exact safe InvalidRequest template selection with no raw path leakage in errors.

## Risks / unknowns

- **Risk**: resolving add-dirs too early could use the wrong working directory source.
- **De-risk plan**: thread the effective working directory explicitly into the helper from the
  backend adapter layer that already owns run defaults.

## Rollout / safety

- Land the shared helper in `backend_harness/normalize.rs` first so backend seams consume one pinned
  `Vec<PathBuf>` contract.
