# Seam map — Universal extra context roots (`agent_api.exec.add_dirs.v1`)

Primary axis: **integration-first (risk-first)** — one core extension key, shared normalization,
and per-backend session-safe mapping.

This document is the complete execution map for the pack. Each seam entry below includes every
required deliverable that must land for the seam to be considered complete, including canonical
backend contract-doc updates, the generated capability artifact, and the final integration closeout
that is folded into SEAM-5 rather than tracked as a standalone orphan workstream.

## Seams

1) **SEAM-1 — Add-dir contract + normalization semantics**
   - Owns (pack/workstream): implementing the pinned v1 meaning of `agent_api.exec.add_dirs.v1`
     as specified by the canonical owner doc `docs/specs/universal-agent-api/extensions-spec.md`.
   - Scope includes: schema, bounds, path-resolution rules, safe error posture, and session-flow
     compatibility requirements.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-add-dirs/seam-1-add-dir-contract-and-normalization.md`
     - contract confirmation in `docs/specs/universal-agent-api/extensions-spec.md`

2) **SEAM-2 — Shared `agent_api` add-dir normalizer**
   - Owns: reusable request parsing/validation/resolution logic that turns the extension payload
     into one normalized unique directory list that backend seams can consume.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-add-dirs/seam-2-shared-agent-api-normalizer.md`
     - `crates/agent_api/src/backend_harness/normalize.rs`
     - exported normalized list contract: `Vec<PathBuf>`

3) **SEAM-3 — Codex backend support**
   - Owns: Codex capability advertising, policy extraction, and mapping of the normalized add-dir
     set across exec, resume, and fork behavior.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-add-dirs/seam-3-codex-mapping.md`
     - `docs/specs/codex-streaming-exec-contract.md` updated with the exec/resume add-dir mapping
     - `docs/specs/codex-app-server-jsonrpc-contract.md` confirmed as the evidence-owned fork
       rejection contract for accepted add-dir inputs
     - updates to `crates/agent_api/src/backends/codex/**`
   - Done when:
     - exec/resume map the accepted normalized directory set with repeated `--add-dir <DIR>` pairs,
     - fork uses the pinned pre-handle backend rejection path only after add-dir capability gating
       and pre-spawn validation succeed, and
     - selector `"last"` and selector `"id"` share the same pre-request fork rejection boundary.

4) **SEAM-4 — Claude Code backend support**
   - Owns: Claude Code capability advertising, policy extraction, and mapping of the normalized
     add-dir set across print/resume/fork behavior.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-add-dirs/seam-4-claude-code-mapping.md`
     - `docs/specs/claude-code-session-mapping-contract.md` confirmed as the evidence-owned resume
       and fork argv-placement contract
     - updates to `crates/agent_api/src/backends/claude_code/**`
   - Done when:
     - fresh-run, resume, and fork flows all preserve the accepted normalized directory set,
     - the Claude selector `"last"` / `"id"` resume and fork branches each have explicit argv
       placement coverage obligations, and
     - any backend-owned runtime failure after handle creation follows the pinned event/completion
       parity rule instead of silently dropping add-dir inputs.

5) **SEAM-5 — Tests + integration closeout**
   - Owns: regression coverage for R0 gating, normalization, pre-spawn filesystem checks,
     no-flag absence behavior, backend argv shape, session-flow parity, capability publication, and
     the final integration closeout after SEAM-3 and SEAM-4 land.
   - Inputs required before closeout:
     - SEAM-2 shared normalizer behavior is pinned and consumable by both backends.
     - SEAM-3 Codex mapping + fork rejection behavior is landed.
     - SEAM-4 Claude mapping + selector-branch behavior is landed.
   - Outputs:
     - `docs/project_management/packs/active/agent-api-add-dirs/seam-5-tests.md`
     - updates/additions to tests under `crates/agent_api/src/**`
     - regenerated `docs/specs/universal-agent-api/capability-matrix.md`
     - same-change integration evidence for `cargo run -p xtask -- capability-matrix`,
       `make test`, and final `make preflight`
   - Done when:
     - Claude resume selector `"last"` / `"id"` and fork selector `"last"` / `"id"` each have an
       explicit acceptance check,
     - Codex fork proves the accepted-input rejection boundary for both selector branches plus the
       invalid-input precedence rule,
     - handle-returning surfaces cover post-handle runtime rejection parity, and
     - the capability matrix regeneration and final `make preflight` close out the pack.
