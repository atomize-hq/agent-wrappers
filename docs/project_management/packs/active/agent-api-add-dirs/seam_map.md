# Seam map — Universal extra context roots (`agent_api.exec.add_dirs.v1`)

Primary axis: **integration-first (risk-first)** — one core extension key, shared normalization,
and per-backend session-safe mapping.

## Seams

1) **SEAM-1 — Add-dir contract + normalization semantics**
   - Owns: the pinned meaning of `agent_api.exec.add_dirs.v1`, including schema, bounds,
     path-resolution rules, safe error posture, and session-flow compatibility requirements.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-add-dirs/seam-1-add-dir-contract-and-normalization.md`
     - contract confirmation in `docs/specs/universal-agent-api/extensions-spec.md`

2) **SEAM-2 — Shared `agent_api` add-dir normalizer**
   - Owns: reusable request parsing/validation/resolution logic that turns the extension payload
     into one normalized unique directory list that backend seams can consume.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-add-dirs/seam-2-shared-agent-api-normalizer.md`
     - shared normalization code under `crates/agent_api/src/backends/` or
       `crates/agent_api/src/backend_harness/`

3) **SEAM-3 — Codex backend support**
   - Owns: Codex capability advertising, policy extraction, and mapping of the normalized add-dir
     set across exec, resume, and fork behavior.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-add-dirs/seam-3-codex-mapping.md`
     - updates to `crates/agent_api/src/backends/codex/**`

4) **SEAM-4 — Claude Code backend support**
   - Owns: Claude Code capability advertising, policy extraction, and mapping of the normalized
     add-dir set across print/resume/fork behavior.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-add-dirs/seam-4-claude-code-mapping.md`
     - updates to `crates/agent_api/src/backends/claude_code/**`

5) **SEAM-5 — Tests**
   - Owns: regression coverage for R0 gating, normalization, pre-spawn filesystem checks,
     no-flag absence behavior, backend argv shape, and session-flow parity.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-add-dirs/seam-5-tests.md`
     - updates/additions to tests under `crates/agent_api/src/**`
