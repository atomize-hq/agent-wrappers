# Seam map — Universal model selection (`agent_api.config.model.v1`)

Primary axis: **integration-first (risk-first)** — one universal key across two backends, with deterministic validation
and backend-owned runtime rejection.

## Seams

1) **SEAM-1 — Core extension key contract**
   - Owns: the verification/sync envelope around the already-pinned normative definition of
     `agent_api.config.model.v1`, including schema, trimmed-value semantics, absence semantics, and the split between
     pre-spawn validation and backend-owned runtime rejection.
   - Status:
     - canonical owner-spec semantics are already landed in `docs/specs/universal-agent-api/extensions-spec.md`
     - this seam remains open only for ADR/pack sync and any canonical-doc drift fixes discovered during verification
   - Outputs:
     - `docs/project_management/packs/active/agent-api-model-selection/seam-1-core-extension-contract.md`
     - updates to `docs/specs/universal-agent-api/extensions-spec.md` only if the verification pass finds unresolved
       drift
     - any required clarifications in `docs/specs/universal-agent-api/run-protocol-spec.md` or
       `docs/specs/universal-agent-api/contract.md`

2) **SEAM-2 — Backend advertising + normalization hook**
   - Owns: built-in backend capability advertising for `agent_api.config.model.v1`, the shared normalization helper
     that turns the raw extension value into the effective trimmed model id before any spawn, and capability-matrix
     publication when advertising changes.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-model-selection/seam-2-backend-advertising-normalization.md`
     - regenerated `docs/specs/universal-agent-api/capability-matrix.md` in the same change that updates built-in
       advertising
     - updates to `crates/agent_api/src/backends/codex/backend.rs`
     - updates to `crates/agent_api/src/backends/claude_code/backend.rs`
     - shared normalization code in `crates/agent_api/src/backend_harness/normalize.rs`

3) **SEAM-3 — Codex backend mapping**
   - Owns: mapping the normalized universal key to Codex builder / argv behavior and preserving safe runtime error
     translation when the selected model cannot be honored.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-model-selection/seam-3-codex-mapping.md`
     - updates to `docs/specs/codex-streaming-exec-contract.md`
     - updates to `docs/specs/codex-app-server-jsonrpc-contract.md`
     - updates to `crates/agent_api/src/backends/codex/harness.rs`
     - updates to `crates/agent_api/src/backends/codex/exec.rs` and `crates/agent_api/src/backends/codex/fork.rs` to
       plumb the normalized `Option<String>` into the existing Codex builder/argv emission and enforce the pinned fork
       rejection path for accepted model-selection inputs
     - if Codex mapping does not live in those modules in the current tree, the SEAM-3 owner MUST update this seam map
       in the same change that wires the mapping so the touched-file set remains explicit and reviewable

4) **SEAM-4 — Claude Code backend mapping**
   - Owns: mapping the normalized universal key to Claude Code print-mode builder / argv behavior while explicitly
     excluding `--fallback-model` and preserving safe runtime error translation.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-model-selection/seam-4-claude-code-mapping.md`
     - updates to `docs/specs/claude-code-session-mapping-contract.md`
     - updates to `crates/agent_api/src/backends/claude_code/harness.rs`
     - updates to `crates/agent_api/src/backends/claude_code/mapping.rs` to plumb the normalized `Option<String>` into
       the existing Claude Code print/session argv emission while explicitly excluding `--fallback-model`
     - if Claude Code mapping does not live in that module in the current tree, the SEAM-4 owner MUST update this seam
       map in the same change that wires the mapping so the touched-file set remains explicit and reviewable

5) **SEAM-5 — Tests**
   - Owns: regression coverage for R0 gating, schema/bounds validation, trim-before-map semantics, absence semantics,
     backend runtime rejection translation, terminal error-event behavior, and verification that the published
     capability matrix matches the landed advertising change.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-model-selection/seam-5-tests.md`
     - updates/additions to tests under `crates/agent_api/src/backend_harness/**`
     - updates/additions to tests under `crates/agent_api/src/backends/{codex,claude_code}/tests/**`
