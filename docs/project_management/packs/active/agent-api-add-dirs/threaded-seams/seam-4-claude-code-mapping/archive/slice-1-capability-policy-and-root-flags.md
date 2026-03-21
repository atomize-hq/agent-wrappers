# S1 — Capability, policy extraction, and root-flags mapping

- **User/system value**: Claude fresh-run surfaces advertise and accept
  `agent_api.exec.add_dirs.v1`, turn the shared normalized directory list into one pinned
  variadic `--add-dir <DIR...>` group, and preserve absent-key behavior.
- **Scope (in/out)**:
  - In:
    - Unconditional capability + allowlist enablement for `agent_api.exec.add_dirs.v1` in the
      built-in Claude backend.
    - Claude policy extraction consumes SEAM-2’s normalized `Vec<PathBuf>` and carries it forward
      as backend policy state.
    - Fresh-run argv construction emits one `--add-dir <DIR...>` group in the root-flags region.
    - Backend-local conformance docs and unit assertions for fresh-run ordering.
  - Out:
    - Resume/fork selector-branch proofs.
    - Post-handle runtime rejection parity.
    - Capability-matrix regeneration and shared fake-runtime scenarios.
- **Acceptance criteria**:
  - `capabilities().ids` and the harness allowlist both include `agent_api.exec.add_dirs.v1`.
  - `validate_and_extract_policy(...)` calls the shared add-dir helper once and stores the
    resulting `Vec<PathBuf>` on Claude policy state.
  - Fresh-run argv emits exactly one `--add-dir <DIR...>` group, keeps normalized order, and does
    not emit the flag when the key is absent.
  - The add-dir group stays after any accepted `--model` pair and before the final `--verbose`
    token and prompt.
- **Dependencies**:
  - Blocked by: SEAM-1 (`AD-C01`, `AD-C03`, `AD-C07`), SEAM-2 (`AD-C02`)
  - Unblocks: S2, SEAM-5
- **Verification**:
  - Backend capability assertions in `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`
  - Backend-local ordering/assertion coverage for fresh-run argv
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Keep the change fail-closed by consuming the shared helper and preserving absent-key
    no-flag behavior.
  - Do not advertise the key until policy extraction and root-flags mapping land in the same PR.

#### S1.T1 — Enable the Claude capability surface for `agent_api.exec.add_dirs.v1`

- **Outcome**: the built-in Claude backend publishes the key in both
  `supported_extension_keys()` and `capabilities().ids`, making R0 gating authoritative.
- **Inputs/outputs**:
  - Inputs: `AD-C01`, existing Claude capability surfaces in
    `crates/agent_api/src/backends/claude_code/mod.rs`,
    `crates/agent_api/src/backends/claude_code/backend.rs`
  - Outputs: capability ids and allowlists that include `agent_api.exec.add_dirs.v1`, plus
    backend-local assertions in `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`
- **Implementation notes**:
  - Add the extension key alongside the existing Claude session keys rather than behind a new
    backend-local opt-in.
  - Keep capability publication tied to the same PR as policy extraction so the backend never
    advertises a key it still ignores.
- **Acceptance criteria**:
  - Default Claude backend construction reports the key in capabilities.
  - Harness-level supported keys include the same key, so unsupported-key failures remain owned by
    the shared R0 gate instead of backend-local branching.
- **Test notes**:
  - Extend the capability suite to assert presence of the new extension id.
- **Risk/rollback notes**:
  - Avoid introducing a second source of truth for supported-extension keys.

Checklist:
- Implement: add the new extension id to Claude capability/allowlist surfaces.
- Test: extend `tests/capabilities.rs` to assert the advertised key is present.
- Validate: confirm capability and supported-key surfaces stay in sync.
- Cleanup: remove any temporary gating or TODOs once the key is live.

#### S1.T2 — Extract the normalized add-dir set into Claude policy state

- **Outcome**: `ClaudeExecPolicy` carries the shared normalized `Vec<PathBuf>` computed from the
  run’s effective working directory, and downstream Claude spawn code stops touching the raw
  extension payload.
- **Inputs/outputs**:
  - Inputs: `AD-C02`, `AD-C03`, request/default working-dir precedence from the scope brief,
    `crates/agent_api/src/backends/claude_code/harness.rs`
  - Outputs: an `add_dirs: Vec<PathBuf>` field on `ClaudeExecPolicy` plus the glue needed to feed
    the effective working directory into `normalize_add_dirs_v1(...)`
- **Implementation notes**:
  - Use the same effective working directory the Claude run will actually use:
    request `working_dir` -> backend `default_working_dir` -> captured run-start cwd.
  - Call the shared helper exactly once in `validate_and_extract_policy(...)` and propagate the
    normalized list forward on policy state.
  - Invalid/missing/non-directory inputs stay on the shared `InvalidRequest` path from SEAM-1/2;
    Claude must not invent backend-local path error text.
- **Acceptance criteria**:
  - Absent key yields `Vec::new()` and no additional backend-local defaults.
  - Relative paths resolve against the effective working directory Claude will use for the run.
  - Downstream spawn code reads only the policy field, not
    `request.extensions["agent_api.exec.add_dirs.v1"]`.
- **Test notes**:
  - Add backend-local unit coverage for absent-key extraction and effective-working-dir handoff.
- **Risk/rollback notes**:
  - The highest-risk failure here is resolving against the wrong cwd; keep the handoff explicit in
    code and tests.

Checklist:
- Implement: extend `ClaudeExecPolicy` with normalized add dirs and wire in the SEAM-2 helper.
- Test: cover absent-key and effective-working-dir cases in Claude backend unit tests.
- Validate: confirm invalid add-dir inputs still surface the shared safe `InvalidRequest` shapes.
- Cleanup: delete any backend-local raw-extension reads once the policy field is wired end to end.

#### S1.T3 — Map fresh-run root flags to one variadic `--add-dir <DIR...>` group

- **Outcome**: fresh Claude runs emit exactly one variadic add-dir group in pinned order and the
  backend contract doc states that root-flags truth explicitly.
- **Inputs/outputs**:
  - Inputs: `AD-C06`, `AD-C07`, `ClaudePrintRequest::add_dirs(...)`, existing model and
    non-interactive ordering in `crates/agent_api/src/backends/claude_code/harness.rs`
  - Outputs: `print_req.add_dirs(...)` wiring plus a fresh-run ordering note in
    `docs/specs/claude-code-session-mapping-contract.md`
- **Implementation notes**:
  - Call `ClaudePrintRequest::add_dirs(...)` once with the normalized list; do not emit repeated
    `--add-dir` flags.
  - Preserve existing root-flag ordering: any accepted `--model` pair remains earlier, then the
    add-dir group, then session selectors / final `--verbose` / prompt.
  - Keep absent-key flows on the no-flag path by passing an empty list only when the key is absent.
- **Acceptance criteria**:
  - Fresh-run argv contains one `--add-dir` token followed by all normalized directories in order.
  - No `--add-dir` token appears when the key is absent.
  - The canonical Claude session-mapping contract describes the fresh-run/root-flags placement in
    the same PR.
- **Test notes**:
  - Add/extend backend-local argv-order assertions; do not pull in SEAM-5’s fake-runtime fixtures.
- **Risk/rollback notes**:
  - Preserve the existing `--permission-mode`, external-sandbox, and model ordering while inserting
    the new group.

Checklist:
- Implement: wire `ClaudePrintRequest::add_dirs(...)` in the fresh-run path.
- Test: add backend-local ordering assertions for model/add-dir/verbose/prompt placement.
- Validate: confirm the generated doc text matches the emitted argv shape.
- Cleanup: remove stale comments that imply Claude does not support add dirs.
