# SEAM-5 — Tests (threaded decomposition)

> Pack: `docs/project_management/packs/active/agent-api-add-dirs/`
> Seam brief: `seam-5-tests.md`
> Threading source of truth: `threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-5
- **Name**: add-dir regression coverage
- **Goal / value**: prove `agent_api.exec.add_dirs.v1` behaves the same across shared
  normalization, backend capability advertising, argv mapping, session selectors, and runtime
  failure handling so hosts can rely on one deterministic cross-backend contract.
- **Type**: risk
- **Scope**
  - In:
    - Shared normalizer tests for AD-C02/AD-C03/AD-C07:
      - absent-key behavior,
      - trim + resolve + lexical normalize + dedup semantics,
      - relative paths against the effective working directory,
      - missing/non-directory failure coverage with exact safe `InvalidRequest` templates,
      - no raw path leakage.
    - Backend regression tests for:
      - built-in capability publication,
      - absence semantics (`--add-dir` omitted when the key is absent),
      - Codex exec/resume argv order and repeated-pair shape,
      - Claude fresh/resume/fork argv order and selector-branch-specific placement,
      - Codex fork accepted-input rejection boundary and invalid-input precedence.
    - Dedicated runtime-rejection parity coverage for every pinned handle-returning surface.
    - Capability-matrix regeneration and the final integration closeout after SEAM-3/4 land.
  - Out:
    - end-to-end live CLI smoke tests.
- **Touch surface**:
  - `crates/agent_api/src/backend_harness/normalize/tests.rs`
  - `crates/agent_api/src/backends/codex/tests/**`
  - `crates/agent_api/src/backends/claude_code/tests/**`
  - `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`
  - `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`
  - `docs/specs/universal-agent-api/capability-matrix.md`
- **Verification**:
  - Targeted runs while iterating:
    - `cargo test -p agent_api` for shared normalizer-only coverage
    - `cargo test -p agent_api --all-features` for backend mapping + session-flow coverage
  - Full gate before merge:
    - `cargo run -p xtask -- capability-matrix`
    - `make test`
    - `make preflight`
- **Threading constraints**
  - Upstream blockers: SEAM-2, SEAM-3, SEAM-4
  - Downstream blocked seams: none
  - Contracts produced (owned): none; SEAM-5 pins conformance to upstream-owned contracts
  - Contracts consumed: AD-C02, AD-C03, AD-C04, AD-C05, AD-C06, AD-C07, AD-C08

## Seam-local slicing strategy

- **Strategy**: risk-first.
- Start by pinning the highest-regression shared rules (`normalize_add_dirs_v1(...)`, safe invalid
  messages, and effective-working-directory handoff), then lock in backend branch behavior, then
  finish with post-handle runtime rejection parity plus the generated capability artifact and full
  gate.

## Slice index

- `S1` → `slice-1-shared-normalizer-validation.md`: pin shared normalization, exact safe validation
  failures, and effective-working-directory handoff.
- `S2` → `slice-2-backend-argv-and-selector-parity.md`: pin backend capability publication, argv
  shape, absence semantics, selector branches, and the Codex fork precedence boundary.
- `S3` → `slice-3-runtime-rejection-and-integration-closeout.md`: pin runtime-rejection parity,
  regenerate the capability matrix, and close out the pack with the required test gates.

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - None. SEAM-5 adds regression coverage and generated artifacts only.
- **Contracts consumed**:
  - `AD-C02` (SEAM-2): effective add-dir set algorithm.
    - Consumed by: `S1.T1` (shared normalizer contract tests) and `S1.T2` (backend
      effective-working-directory handoff tests).
  - `AD-C03` (SEAM-1): safe error posture and exact `InvalidRequest` templates.
    - Consumed by: `S1.T1` (exact template + no-leak assertions) and `S3.T2` (runtime error
      redaction parity).
  - `AD-C04` (SEAM-1): session-flow parity for new/resume/fork surfaces.
    - Consumed by: `S2.T2` (Codex fork branch coverage) and `S2.T3` (Claude resume/fork branch
      coverage).
  - `AD-C05` (SEAM-3): Codex repeated `--add-dir <DIR>` mapping after any accepted `--model` pair.
    - Consumed by: `S2.T1` (Codex exec/resume argv coverage).
  - `AD-C06` (SEAM-4): Claude variadic `--add-dir <DIR...>` group after `--model` and before
    `--continue` / `--fork-session` / `--resume` / final `--verbose`.
    - Consumed by: `S2.T3` (Claude fresh/resume/fork placement coverage).
  - `AD-C07` (SEAM-1): absence semantics (`Ok(Vec::new())`; no emitted argv when absent).
    - Consumed by: `S1.T1` (helper absence behavior) and `S2.T1` / `S2.T3` (backend no-flag
      assertions).
  - `AD-C08` (SEAM-3): Codex fork validation-vs-rejection precedence.
    - Consumed by: `S2.T2` (accepted-input rejection-before-request tests and invalid-input
      precedence tests).
- **Dependency edges honored**:
  - `SEAM-2 blocks SEAM-5`: `S1` waits for the shared helper and backend policy handoff to be
    stable before pinning exact semantics.
  - `SEAM-3 blocks SEAM-5`: `S2.T1` / `S2.T2` and the Codex half of `S3` depend on final Codex
    capability, argv, and fork-rejection behavior.
  - `SEAM-4 blocks SEAM-5`: `S2.T3` and the Claude half of `S3` depend on final Claude capability,
    argv placement, and runtime-rejection behavior.
- **Parallelization notes**:
  - What can proceed now: `S1` can land as soon as SEAM-2 exposes the final normalizer and backend
    policy extraction path.
  - What can proceed in parallel later: `S2.T1` / `S2.T2` (Codex) and `S2.T3` (Claude) can run in
    parallel once SEAM-3 and SEAM-4 land.
  - What must wait: `S3.T2` depends on dedicated runtime-rejection fixtures from `S3.T1`, and
    `S3.T3` must be the last PR in the seam because it regenerates the capability matrix and runs
    the full gate.
