# Seam map — Universal model selection (`agent_api.config.model.v1`)

Primary axis: **integration-first (risk-first)** — one universal key across two backends, with deterministic validation
and backend-owned runtime rejection.

## Seams

1) **SEAM-1 — Core extension key contract**
   - Owns: the normative definition of `agent_api.config.model.v1`, including schema, trimmed-value semantics, absence
     semantics, and the split between pre-spawn validation and backend-owned runtime rejection.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-model-selection/seam-1-core-extension-contract.md`
     - updates to `docs/specs/universal-agent-api/extensions-spec.md`
     - any required clarifications in `docs/specs/universal-agent-api/run-protocol-spec.md` or
       `docs/specs/universal-agent-api/contract.md`

2) **SEAM-2 — Backend advertising + normalization hook**
   - Owns: built-in backend capability advertising for `agent_api.config.model.v1`, plus the shared or mirrored
     normalization path that turns the raw extension value into the effective trimmed model id before any spawn.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-model-selection/seam-2-backend-advertising-normalization.md`
     - updates to `crates/agent_api/src/backends/codex/backend.rs`
     - updates to `crates/agent_api/src/backends/claude_code/backend.rs`
     - shared or backend-local normalization code under `crates/agent_api/src/backends/**` and/or
       `crates/agent_api/src/backend_harness/**`

3) **SEAM-3 — Codex backend mapping**
   - Owns: mapping the normalized universal key to Codex builder / argv behavior and preserving safe runtime error
     translation when the selected model cannot be honored.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-model-selection/seam-3-codex-mapping.md`
     - updates to `crates/agent_api/src/backends/codex/harness.rs`
     - updates to `crates/agent_api/src/backends/codex/exec.rs`, `fork.rs`, or related Codex request mapping code as needed

4) **SEAM-4 — Claude Code backend mapping**
   - Owns: mapping the normalized universal key to Claude Code print-mode builder / argv behavior while explicitly
     excluding `--fallback-model` and preserving safe runtime error translation.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-model-selection/seam-4-claude-code-mapping.md`
     - updates to `crates/agent_api/src/backends/claude_code/harness.rs`
     - updates to `crates/agent_api/src/backends/claude_code/mapping.rs` or related request mapping code as needed

5) **SEAM-5 — Tests**
   - Owns: regression coverage for R0 gating, schema/bounds validation, trim-before-map semantics, absence semantics,
     backend runtime rejection translation, and terminal error-event behavior.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-model-selection/seam-5-tests.md`
     - updates/additions to tests under `crates/agent_api/src/backend_harness/**`
     - updates/additions to tests under `crates/agent_api/src/backends/{codex,claude_code}/tests/**`
