# SEAM-5 — Tests (threaded decomposition)

> Pack: `docs/project_management/packs/active/agent-api-model-selection/`
> Seam brief: `seam-5-tests.md`
> Threading source of truth: `threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-5
- **Name**: regression coverage for `agent_api.config.model.v1`
- **Goal / value**: lock the universal model-selection contract in place so backend or spec churn
  cannot silently regress R0 ordering, trim-before-map semantics, absence behavior, backend argv
  placement, safe runtime rejection, or capability publication.
- **Type**: risk (contract conformance + regression)
- **Scope**
  - In:
    - `SEAM-5A` coverage for unsupported-before-`InvalidRequest` ordering and the pinned safe
      invalid template `invalid agent_api.config.model.v1`.
    - `SEAM-5B` coverage for trim-before-map semantics, absence/default preservation, Codex and
      Claude backend mapping, runtime rejection translation, terminal `Error` event emission, and
      capability-matrix freshness after advertising changes.
    - Regression checks that backend seams consume the shared normalized value rather than
      re-reading the raw extension payload.
  - Out:
    - live upstream catalog/e2e compatibility tests against real model availability.
    - speculative tests for future universal keys such as `fallback-model`.
- **Touch surface**:
  - `crates/agent_api/src/backend_harness/normalize/tests.rs`
  - `crates/agent_api/src/backend_harness/runtime/tests.rs`
  - `crates/agent_api/src/backend_harness/runtime/tests/**`
  - `crates/agent_api/src/backends/codex/tests/capabilities.rs`
  - `crates/agent_api/src/backends/codex/tests/mapping.rs`
  - `crates/agent_api/src/backends/codex/tests/app_server.rs`
  - `crates/agent_api/src/backends/codex/tests/backend_contract.rs`
  - `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`
  - `crates/agent_api/src/backends/claude_code/tests/mapping.rs`
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`
  - `docs/specs/universal-agent-api/capability-matrix.md`
- **Verification**:
  - `cargo test -p agent_api backend_harness::normalize`
  - `cargo test -p agent_api codex`
  - `cargo test -p agent_api claude_code`
  - `cargo run -p xtask -- capability-matrix`
  - `make test`
- **Threading constraints**
  - Upstream blockers:
    - `SEAM-1` unlocks the early `SEAM-5A` ordering and invalid-template tests.
    - `SEAM-2` unlocks shared-normalizer handoff coverage and capability-matrix freshness
      assertions.
    - `SEAM-3` unlocks Codex mapping and Codex runtime-rejection coverage.
    - `SEAM-4` unlocks Claude mapping and Claude runtime-rejection coverage.
  - Downstream blocked seams:
    - none; this seam is the terminal regression gate for the feature.
  - Contracts produced (owned):
    - none. SEAM-5 only pins conformance to contracts owned by SEAM-1 through SEAM-4.
  - Contracts consumed:
    - `MS-C03`, `MS-C04`, `MS-C05`, `MS-C06`, `MS-C07`, `MS-C08`, `MS-C09`

## Seam-local slicing strategy

- **Strategy**: dependency-first.
- **Why**: the seam already has an explicit `SEAM-5A` / `SEAM-5B` split in the brief and
  `threading.md`. Land the pre-spawn ordering guardrails first, then layer backend-specific mapping
  and runtime/publication conformance once SEAM-2/3/4 have stabilized.

## Slice index

- `S1` → `slice-1-pre-spawn-validation-ordering.md`: land the early `SEAM-5A` guardrails for R0
  ordering, trim-first validation, and the exact safe `InvalidRequest` template.
- `S2` → `slice-2-backend-mapping-and-absence.md`: pin deterministic backend mapping, absence
  semantics, fork/fallback exclusions, and the no-second-parser handoff.
- `S3` → `slice-3-runtime-rejection-and-publication-guardrails.md`: pin safe runtime rejection,
  terminal `Error` event behavior, and capability-matrix freshness after advertising flips.

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - None. SEAM-5 contributes regression tests only; it does not own new contracts.
- **Contracts consumed**:
  - `MS-C03` (SEAM-1): pre-spawn validation schema, trim-first semantics, and exact safe
    `InvalidRequest` template.
    - Consumed by: `S1.T1` and `S1.T2`.
  - `MS-C09` (SEAM-2): shared `Result<Option<String>, AgentWrapperError>` normalizer handoff with
    no backend-local raw parsing.
    - Consumed by: `S2.T1`, `S2.T2`, and `S2.T3`.
  - `MS-C05` (SEAM-2): built-in advertising is allowed only when every exposed flow is
    deterministic, and matrix publication must move with that advertising change.
    - Consumed by: `S2.T1`, `S2.T2`, and `S3.T3`.
  - `MS-C06` (SEAM-3): Codex exec/resume emit exactly one `--model <trimmed-id>` and fork uses
    the pinned safe pre-handle rejection path.
    - Consumed by: `S2.T1` and `S3.T1`.
  - `MS-C07` (SEAM-4): Claude emits exactly one `--model <trimmed-id>` before add-dir, session,
    and fallback flags; the universal key never drives `--fallback-model`.
    - Consumed by: `S2.T2` and `S3.T2`.
  - `MS-C04` (SEAM-1): syntactically valid but backend-rejected model ids surface as safe
    `Backend` errors; if the stream is already open, emit exactly one terminal
    `AgentWrapperEventKind::Error` with the same safe message.
    - Consumed by: `S3.T1` and `S3.T2`.
  - `MS-C08` (SEAM-2): capability-matrix regeneration happens in the same change that changes
    advertising, and stale matrix diffs block merge.
    - Consumed by: `S3.T3`.
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-5A`: `S1` contains only the early ordering/template coverage that should
    land first.
  - `SEAM-2 blocks SEAM-5B`: `S2` and `S3` assume the shared normalizer and advertising handoff
    already exist.
  - `SEAM-3 blocks SEAM-5B`: Codex mapping/runtime tasks stay isolated to `S2.T1` and `S3.T1`.
  - `SEAM-4 blocks SEAM-5B`: Claude mapping/runtime tasks stay isolated to `S2.T2` and `S3.T2`.
- **Parallelization notes**:
  - What can proceed now:
    - `S1` once SEAM-1 verification is complete.
  - What must wait:
    - `S2` waits on SEAM-2 plus the relevant backend mapping seam (`SEAM-3` or `SEAM-4`).
    - `S3` waits on SEAM-2/3/4 because it verifies the final backend-owned runtime behavior and
      published matrix artifact.
