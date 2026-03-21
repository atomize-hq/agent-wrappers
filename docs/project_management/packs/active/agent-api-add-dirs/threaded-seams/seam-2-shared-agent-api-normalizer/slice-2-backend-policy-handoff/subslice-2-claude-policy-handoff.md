### S2b — Carry normalized add-dirs through Claude policy extraction

- **User/system value**: Claude matches Codex on effective cwd ownership and add-dir normalization,
  which gives downstream Claude mapping work one policy field to consume instead of backend-local
  raw payload parsing.
- **Scope (in/out)**:
  - In:
    - Capture `run_start_cwd` in Claude backend entrypoints before constructing the harness
      adapter.
    - Add `add_dirs: Vec<PathBuf>` to `ClaudeExecPolicy`.
    - Compute Claude effective cwd inside
      `ClaudeHarnessAdapter::validate_and_extract_policy(...)`.
    - Call the shared helper exactly once from Claude policy extraction and attach the normalized
      list to policy state.
    - Add Claude-only direct-policy tests covering absence, cwd precedence, valid relative
      resolution, and safe invalid propagation.
  - Out:
    - Claude capability / supported-extension allowlist work owned by SEAM-4.
    - Claude argv placement, print-request mapping, or selector-branch assertions owned by later
      seams.
    - Runtime rejection fixtures or capability-matrix regeneration.
- **Acceptance criteria**:
  - `run(...)` and `run_control(...)` capture `std::env::current_dir().ok()` and feed it into the
    harness adapter as `run_start_cwd`.
  - `ClaudeExecPolicy` carries normalized `add_dirs: Vec<PathBuf>`.
  - Claude effective cwd precedence is
    `request.working_dir -> config.default_working_dir -> run_start_cwd` before helper invocation.
  - `ClaudeHarnessAdapter::validate_and_extract_policy(...)` calls the shared helper exactly once
    and returns `Vec::new()` when the key is absent.
  - Claude direct-policy tests prove run-start cwd participates only as the final fallback and that
    safe invalid helper errors propagate unchanged.
- **Dependencies**:
  - `S1`
  - `AD-C02`
  - `AD-C04`
  - `SEAM-1`
- **Verification**:
  - `cargo test -p agent_api claude_code`
  - Direct policy-extraction tests under `crates/agent_api/src/backends/claude_code/tests/`
- **Rollout/safety**:
  - Keep this sub-slice limited to backend entrypoint capture, policy extraction, and direct-policy
    tests; do not mix in Claude capability or argv work.

## Feature Brief

- **Goal**: make Claude policy extraction own the normalized add-dir handoff before SEAM-4 touches
  argv placement.
- **Why now**:
  - Claude currently has no `run_start_cwd` field on the harness adapter, so policy extraction
    cannot match the pack’s pinned effective-working-directory ladder.
  - `ClaudeExecPolicy` currently stops at non-interactive, external-sandbox, resume, and fork
    state, which leaves no backend policy surface for normalized add dirs.
  - The current Claude tests cover capability, session-handle, mapping, and external-sandbox
    behavior, but there is no direct-policy module proving add-dir attachment or cwd precedence.
- **In scope**:
  - `crates/agent_api/src/backends/claude_code/backend.rs`
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backends/claude_code/tests/support.rs`
  - `crates/agent_api/src/backends/claude_code/tests/policy_add_dirs.rs`
  - `crates/agent_api/src/backends/claude_code/tests/mod.rs`
- **Out of scope**:
  - `capabilities()` and `supported_extension_keys()` publication work
  - `ClaudePrintRequest` construction and `--add-dir` argv ordering
  - Resume / fork selector branch assertions beyond direct policy extraction
- **Constraints**:
  - Preserve the existing Claude parsing order for non-interactive, external-sandbox, resume, and
    fork handling.
  - Keep `run_start_cwd` as policy-extraction metadata only; do not start forcing
    `builder.working_dir(run_start_cwd)` in spawn code because the ambient process cwd already
    represents that fallback.
  - The only raw add-dir payload read in this subslice must be the helper invocation inside
    `validate_and_extract_policy(...)`.

## Implementation Plan

#### S2b.T1 — Thread `run_start_cwd` into Claude harness construction

- **Outcome**: every Claude run surface captures the invocation cwd once and makes it available to
  policy extraction without changing the existing spawn/build pipeline.
- **Inputs/outputs**:
  - Inputs: current `new_harness_adapter(...)` constructor, `run(...)`, and `run_control(...)`
    entrypoints.
  - Outputs: `ClaudeHarnessAdapter { run_start_cwd: Option<PathBuf>, ... }`.
  - Outputs: backend entrypoints that pass `std::env::current_dir().ok()` into adapter
    construction.
  - Outputs: test support helpers that can build an adapter with an explicit `run_start_cwd`.
- **Implementation notes**:
  - Add `PathBuf` import and a `run_start_cwd: Option<PathBuf>` field to `ClaudeHarnessAdapter`.
  - Extend `new_harness_adapter(...)` and `new_test_adapter(...)` to accept the captured cwd.
  - Update `crates/agent_api/src/backends/claude_code/backend.rs` so both `run(...)` and
    `run_control(...)` capture the cwd before constructing the adapter.
  - Add a focused helper in `crates/agent_api/src/backends/claude_code/tests/support.rs` for
    explicit run-start-cwd test setup rather than overloading unrelated test modules.
- **Acceptance criteria**:
  - Claude backend entrypoints capture `current_dir().ok()` exactly once per request path.
  - Test code can instantiate a Claude adapter with `run_start_cwd = Some(path)` or `None`
    without duplicating constructor logic.
  - No spawn-path behavior changes occur yet beyond constructor argument threading.
- **Test notes**:
  - Direct behavioral proof lands in `S2b.T3`; `T1` only needs the support surface required by
    those tests.
- **Risk / rollback**:
  - Constructor churn is the primary risk. Keep all signature updates in the same patch so the
    harness/test call sites stay in sync.

Checklist:
- Implement: add `run_start_cwd: Option<PathBuf>` to `ClaudeHarnessAdapter`.
- Implement: thread the new constructor argument through `new_harness_adapter(...)` and
  `new_test_adapter(...)`.
- Implement: capture `std::env::current_dir().ok()` in `backend.rs` before adapter construction in
  both `run(...)` and `run_control(...)`.
- Implement: add `new_adapter_with_run_start_cwd(...)` support for Claude backend tests.
- Validate: confirm no Claude spawn code starts calling `builder.working_dir(...)` with
  `run_start_cwd`.

#### S2b.T2 — Attach normalized `add_dirs` during Claude policy extraction

- **Outcome**: `ClaudeExecPolicy` becomes the single Claude-owned handoff surface for normalized
  add dirs, with cwd precedence resolved before the shared helper runs.
- **Inputs/outputs**:
  - Inputs: `S1` shared helper export and the pack’s pinned precedence contract from `threading.md`
    and `scope_brief.md`.
  - Outputs: `ClaudeExecPolicy { add_dirs: Vec<PathBuf>, ... }`.
  - Outputs: one helper invocation inside
    `ClaudeHarnessAdapter::validate_and_extract_policy(...)`.
- **Implementation notes**:
  - Import `PathBuf` and add `add_dirs: Vec<PathBuf>` to `ClaudeExecPolicy`.
  - Preserve existing non-interactive and external-sandbox validation plus resume/fork parsing and
    mutual-exclusion checks.
  - After session parsing is complete, resolve the effective cwd with
    `request.working_dir.as_deref() -> self.config.default_working_dir.as_deref() -> self.run_start_cwd.as_deref()`.
  - Call the shared helper exactly once with
    `request.extensions.get("agent_api.exec.add_dirs.v1")` and the selected effective cwd, then
    store the returned vector on policy.
  - Keep downstream Claude spawn/mapping code unchanged in this subslice; later seams consume the
    policy field.
- **Acceptance criteria**:
  - Absent add-dir input yields `policy.add_dirs == Vec::new()`.
  - Relative add-dir input resolves against request cwd first, backend default second, and
    `run_start_cwd` only as the last fallback.
  - Invalid helper results propagate as the shared safe `InvalidRequest` messages with no
    Claude-authored override text.
  - The raw add-dir payload is not reopened elsewhere in Claude backend code as part of this
    change.
- **Test notes**:
  - Cover helper attachment through direct `validate_and_extract_policy(...)` calls, not through
    argv emission or spawned-process assertions.
- **Risk / rollback**:
  - The main drift risk is accidentally reordering existing session parsing around the helper call.
    Keep selector parsing intact and add tests that exercise the new code path without broadening
    scope.

Checklist:
- Implement: add `add_dirs: Vec<PathBuf>` to `ClaudeExecPolicy`.
- Implement: resolve effective cwd in `ClaudeHarnessAdapter::validate_and_extract_policy(...)`
  after the existing selector parsing steps.
- Implement: call the shared helper exactly once and attach the returned vector to policy state.
- Validate: confirm Claude backend files outside policy extraction still contain no raw
  `agent_api.exec.add_dirs.v1` reads.
- Cleanup: keep add-dir handling out of `spawn(...)` for this subslice.

#### S2b.T3 — Add Claude direct-policy regression coverage for add-dir handoff

- **Outcome**: Claude has a dedicated policy-level regression module that pins absence semantics,
  cwd precedence, valid relative attachment, and safe invalid propagation before SEAM-4 reuses the
  policy field.
- **Inputs/outputs**:
  - Inputs: constructor/test support from `S2b.T1` and policy state from `S2b.T2`.
  - Outputs: `crates/agent_api/src/backends/claude_code/tests/policy_add_dirs.rs`.
  - Outputs: `crates/agent_api/src/backends/claude_code/tests/mod.rs` updated to register the new
    module.
- **Implementation notes**:
  - Add one absent-key case asserting `policy.add_dirs == Vec::new()`.
  - Add a request-vs-default precedence case using distinct temp directories and relative add-dir
    input.
  - Add a default-vs-run-start precedence case using `new_adapter_with_run_start_cwd(...)` and a
    config-level default cwd.
  - Add a run-start-only fallback case proving Claude can normalize a relative add-dir when both
    request and backend default are absent.
  - Add at least one invalid-path case that asserts the exact shared safe message family and proves
    the raw path text is not echoed.
- **Acceptance criteria**:
  - The new test module exercises direct policy extraction only.
  - Test fixtures prove the full precedence ladder
    `request -> default -> run_start_cwd`.
  - Invalid add-dir cases fail before any later mapping-layer assertions are needed.
  - The targeted suite passes via `cargo test -p agent_api claude_code`.
- **Test notes**:
  - Use temp directories / temp files so success and non-directory failures are deterministic.
  - Assert on `policy.add_dirs` contents and `AgentWrapperError::InvalidRequest { message }`, not
    on downstream argv state.
- **Risk / rollback**:
  - Filesystem fixtures can be brittle if they leak process cwd assumptions. Use explicit adapter
    cwd injection for the fallback path instead of mutating global cwd in test bodies.

Checklist:
- Implement: add `policy_add_dirs.rs` and register it from `tests/mod.rs`.
- Test: absent key returns `Vec::new()`.
- Test: request cwd beats backend default for relative add-dir resolution.
- Test: backend default beats `run_start_cwd` when both are present.
- Test: `run_start_cwd` resolves relative add dirs when it is the only cwd source.
- Test: invalid add-dir input propagates the shared safe message without raw path leakage.
- Validate: run `cargo test -p agent_api claude_code`.

## Dependency Graph

- `S1` blocks `S2b.T2` because the shared helper signature and safe error posture must exist
  first.
- `S2b.T1` blocks `S2b.T2` because Claude policy extraction cannot use `run_start_cwd` until the
  adapter carries it.
- `S2b.T2` blocks `S2b.T3` because tests need a populated `policy.add_dirs` field to assert on.
- `S2b` blocks `S3c` and downstream SEAM-4 policy consumers because they should assert against
  policy state, not raw extension parsing.

## Risks / Unknowns

- **Risk**: future SEAM-4 work could reintroduce raw add-dir parsing in Claude mapping code.
  - **De-risk**: keep this subslice’s tests policy-only and leave the later drift audit to `S3c`.
- **Risk**: the run-start cwd fallback is easy to misinterpret as a new spawn override.
  - **De-risk**: keep `run_start_cwd` scoped to policy extraction and test support; do not change
    `spawn(...)` builder behavior here.
- **Risk**: constructor changes can spread quietly across tests.
  - **De-risk**: centralize new adapter setup in `tests/support.rs` and avoid ad hoc constructor
    calls in the new module.
- **Assumption**: `std::env::current_dir().ok()` remains available for normal Claude runs, so this
  subslice does not define a new Claude-specific unresolved-working-directory error path.
  - **De-risk**: if implementation work proves that assumption false, stop and pin the failure
    contract in a follow-up planning update before broadening scope.

## Milestones

- **M1**: Claude backend entrypoints and test helpers can supply `run_start_cwd`.
- **M2**: `ClaudeExecPolicy` carries normalized `add_dirs` from a single helper invocation.
- **M3**: Claude direct-policy regressions pin absence semantics, precedence, success-path
  attachment, and safe invalid propagation.
