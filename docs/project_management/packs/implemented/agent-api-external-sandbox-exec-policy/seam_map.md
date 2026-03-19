# Seam map — External sandbox execution policy (dangerous)

Primary axis: **integration-first (risk-first)** — dangerous execution policy + per-backend CLI mapping.

## Seams

1) **SEAM-1 — Core extension key contract**
   - Owns: the normative definition of `agent_api.exec.external_sandbox.v1` and its interaction
     with `agent_api.exec.non_interactive`.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/seam-1-external-sandbox-extension-key.md`
     - updates to `docs/specs/universal-agent-api/extensions-spec.md`

2) **SEAM-2 — Backend enablement + capability advertising**
   - Owns: host opt-in mechanism for built-in backends, and ensuring the capability is not
     advertised by default.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/seam-2-backend-enablement.md`
     - updates to `crates/agent_api/src/backends/codex.rs`
     - updates to `crates/agent_api/src/backends/claude_code.rs`

3) **SEAM-3 — Codex backend mapping**
   - Owns: mapping the key to Codex "danger bypass approvals + sandbox" execution policy while
     remaining non-interactive.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/seam-3-codex-mapping.md`
     - updates to `crates/agent_api/src/backends/codex.rs` (+ `codex/exec.rs`, `codex/fork.rs`)

4) **SEAM-4 — Claude Code backend mapping**
   - Owns: mapping the key to Claude dangerous permission bypass flags while remaining
     non-interactive and deterministic across CLI versions.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/seam-4-claude-code-mapping.md`
     - updates to `crates/agent_api/src/backends/claude_code.rs`

5) **SEAM-5 — Tests**
   - Owns: regression coverage for validation ordering, contradiction behavior, and default
     advertising posture.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/seam-5-tests.md`
     - updates/additions to tests under `crates/agent_api/src/**`

