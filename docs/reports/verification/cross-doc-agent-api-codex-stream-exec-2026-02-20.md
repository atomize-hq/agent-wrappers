# Cross-Documentation Verification Report

**Target**: `agent-api-codex-stream-exec` (docs + CI + `agent_api` implementation)  
**Date (UTC)**: 2026-02-20  
**Changed paths source**: `docs/reports/verification/changed-paths-2026-02-20.md`  
**Workstream queue**: `docs/reports/verification/cross-doc-agent-api-codex-stream-exec-2026-02-20.tasks.json`

## Executive Summary

Docs (ADR-0011/0012, universal baselines, and the feature planning pack) are consistent on the
exec-policy extension surface and ownership rules. The `agent_api` Codex backend now consumes the
`crates/codex` streaming API (`CodexClient::stream_exec_with_env_overrides`) and maps completion
`final_text` from `ExecCompletion.last_message` as pinned. Remaining work is limited to the two
documented gaps (canonical redaction mapping and the required C2 fake-binary scenarios + tests).

Treat this as **NOT DONE** for “implementation complete” until the remaining gaps are filled;
proceed with triads `C1 → C2`.

## Consistency Score: 90/100

- Conflicts: 0
- Gaps: 2
- Duplication: 0
- Drift: 0

## Conflicts (Must Resolve)

- None

## Gaps (Should Fill)

### Gap 1: Redaction mapping in code does not implement the pinned “canonical, deterministic” shapes

- **Location 1**: `docs/project_management/packs/active/agent-api-codex-stream-exec/codex-stream-exec-adapter-protocol-spec.md` (lines 141–160)
- **Location 2**: `crates/agent_api/src/backends/codex.rs` (`redacted_exec_error`)
- **Gap**: Spec requires stable messages including `line_bytes={n}` for Parse/Normalize and stable `{kind}` categories for Codex
  errors; code currently emits simplified strings (and collapses all `ExecStreamError::Codex(_)` to `"codex error"`).
- **Resolution**: Implement `redact_exec_stream_error` per spec and use it for both stream item errors and completion errors.

### Gap 2: C2 fake-binary scenarios + tests required by the pack are not yet present

- **Location 1**: `docs/project_management/packs/active/agent-api-codex-stream-exec/C2-spec.md` (lines 55–92)
- **Location 2**: `crates/agent_api/src/bin/fake_codex_stream_json_agent_api.rs` (no `FAKE_CODEX_SCENARIO`; only argv validation + fixture emit)
- **Location 3**: `crates/agent_api/tests/c1_codex_exec_policy.rs` (exec-policy only; does not cover env precedence/redaction/live-before-completion; filename differs from spec)
- **Gap**: The pack requires scenario selection, an env dump path, and a raw-line redaction sentinel test; current artifacts only
  validate sandbox/approval flags.
- **Resolution**: Add the required scenarios + tests (and align file naming/locations to the spec, or update the spec intentionally).

## Positive Findings

- ✅ Core extension ownership rules are centralized in `docs/project_management/next/universal-agent-api/extensions-spec.md`.
- ✅ `.github/workflows/ci.yml` now includes `cargo test -p agent_api --all-features` (ubuntu-latest) as required by the decision register.
- ✅ `.github/workflows/agent-api-codex-stream-exec-smoke.yml` matches the pack’s platform parity spec (OS matrix + public API guard + Linux preflight).

## Recommendations

1. **BLOCK “done”**: resolve the two conflicts before treating ADR-0011 as implemented.
2. Proceed with the planned triads in order (`C0 → C1 → C2`) to close the two gaps and produce cross-platform evidence.
