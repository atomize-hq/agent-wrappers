# SEAM-2 — Backend advertising + normalization hook

- **Name**: Backend advertising + normalization hook
- **Type**: integration
- **Goal / user value**: Ensure both built-in backends expose the capability consistently and consume one effective
  trimmed model id contract instead of duplicating drift-prone raw extension parsing.
- **Contract registry cross-refs**: MS-C05, MS-C08, MS-C09 (see `threading.md`)
- **Scope**
  - In:
    - add `agent_api.config.model.v1` to built-in backend capability sets once deterministic support exists
    - implement the normalization locus for extracting the effective trimmed model id before spawn
    - keep R0 gating ahead of model parsing/validation
    - make the normalized result available to backend mapping seams
    - regenerate `docs/specs/universal-agent-api/capability-matrix.md` in the same change that flips advertising
  - Out:
    - actual Codex/Claude argv insertion details
    - backend-specific runtime rejection translation
- **Primary interfaces (contracts)**
  - **Shared helper entrypoint**
    - Inputs:
      - canonical key semantics from SEAM-1
      - `request.extensions.get("agent_api.config.model.v1")` after the R0 capability gate has accepted the key
    - Outputs:
      - `Result<Option<String>, AgentWrapperError>` from
        `crates/agent_api/src/backend_harness/normalize.rs`, where:
        - `Ok(None)` means the key is absent,
        - `Ok(Some(trimmed_model_id))` means the key is present and valid, and
        - `Err(AgentWrapperError::InvalidRequest { message: "invalid agent_api.config.model.v1" })` covers
          non-string, empty-after-trim, and oversize-after-trim inputs.
  - **Backend-consumption contract**
    - Inputs:
      - normalized `Option<String>` from the shared helper
      - backend capability sets in `crates/agent_api/src/backends/{codex,claude_code}/backend.rs`
      - existing builder/request APIs in `crates/codex/src/builder/mod.rs` and
        `crates/claude_code/src/commands/print.rs`
    - Outputs:
      - deterministic built-in advertising of `agent_api.config.model.v1`
      - Codex policy/mapping code calls `CodexClientBuilder::model(trimmed_model_id)` only when the helper returns
        `Some(...)`
      - Claude policy/mapping code calls `ClaudePrintRequest::model(trimmed_model_id)` only when the helper returns
        `Some(...)`
      - when the helper returns `None`, neither backend emits `--model`
- **Key invariants / rules**:
  - unsupported key still fails as `UnsupportedCapability` before parser logic runs
  - normalization is trim-first and uses the same bounds on both backends
  - absence is represented explicitly so downstream mapping can omit `--model`
  - normalization must be cheap/local and require no remote lookup
  - backend-local mirrored parsers are not permitted for this key; both built-in backends consume the shared helper in
    `crates/agent_api/src/backend_harness/normalize.rs`
  - the shared helper is the only code allowed to read and validate the raw
    `agent_api.config.model.v1` JSON payload; downstream seams consume only the normalized `Option<String>`
  - backend mapping seams MUST NOT emit `--model` from raw `request.extensions`; they reuse the existing builder/request
    argv emission that already places `--model` before `--add-dir` / session-selector / `--fallback-model` flags
  - backend builders are containers, not validators: `CodexClientBuilder::model(...)` and `ClaudePrintRequest::model(...)`
    receive the already-normalized string and MUST NOT become a second parser for this key
- **Dependencies**
  - Blocks:
    - SEAM-3
    - SEAM-4
    - SEAM-5
  - Blocked by:
    - SEAM-1
- **Touch surface**:
  - `crates/agent_api/src/backends/codex/backend.rs`
  - `crates/agent_api/src/backends/claude_code/backend.rs`
  - `crates/agent_api/src/backends/codex/harness.rs`
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backend_harness/normalize.rs`
  - any new shared helper under `crates/agent_api/src/backends/` if extracted
- **Verification**:
  - built-in capability sets advertise `agent_api.config.model.v1` only when the backend can deterministically honor
    the owner-doc semantics for every exposed run flow after R0 gating and pre-spawn validation: either apply the
    accepted effective trimmed model id unchanged to the backend transport for that flow, or take a pinned
    backend-owned safe rejection path (a flow that silently drops, rewrites, or conditionally ignores an accepted model
    id is not deterministic support). Because `AgentWrapperCapabilities.ids` is backend-global, built-in advertising
    can be unconditional only when every exposed flow has one of those pinned outcomes.
  - R0 ordering tests prove unsupported key fails before model parsing
  - parser tests prove absent / non-string / empty / oversize cases fail or succeed deterministically
  - parser tests prove all invalid cases use the exact safe template `invalid agent_api.config.model.v1`
  - valid trimmed values flow to mapping seams without exposing raw untrimmed values
  - Codex mapping tests prove the helper output reaches `CodexClientBuilder::model(...)` once and that argv order stays:
    wrapper-owned CLI overrides, then exactly one `--model <trimmed-id>` pair, then any capability-guarded `--add-dir`
  - Claude mapping tests prove the helper output reaches `ClaudePrintRequest::model(...)` once and that argv order keeps
    `--model <trimmed-id>` before any `--add-dir` group, session-selector flags, and `--fallback-model`
  - review/CI gate for SEAM-2/3/4 rejects any new direct parsing of `agent_api.config.model.v1` outside
    `crates/agent_api/src/backend_harness/normalize.rs`
  - capability-matrix regeneration occurs in the same change as advertising changes
- **Risks / unknowns**
  - Risk:
    - advertising could drift from the published capability matrix if regeneration is left to a later seam
  - De-risk plan:
    - require one shared helper in `crates/agent_api/src/backend_harness/normalize.rs` and treat stale
      `capability-matrix.md` output as merge-blocking in WS-INT
- **Rollout / safety**:
  - land advertising only alongside working normalization
  - keep the shared helper backend-neutral and limited to extension parsing
  - land the shared helper contract before backend mapping seams so SEAM-3 and SEAM-4 consume one pinned
    `Option<String>` interface instead of inventing backend-local trimming behavior
