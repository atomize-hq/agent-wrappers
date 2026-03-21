### S2 — Backend argv + selector-branch parity

- **User/system value**: proves both built-in backends advertise and honor the accepted add-dir set
  with the exact backend-owned mapping contracts, and that the Codex fork exception stays bounded
  to the pinned pre-request rejection path.
- **Scope (in/out)**:
  - In:
    - Capability publication tests for both built-in backends.
    - Codex exec/resume argv shape + ordering + absence semantics.
    - Codex fork selector `"last"` / `"id"` accepted-input rejection boundary tests.
    - Codex fork invalid-input precedence tests.
    - Claude fresh/resume/fork argv shape + ordering + absence semantics.
    - Claude resume selector `"last"` / `"id"` and fork selector `"last"` / `"id"` placement tests.
  - Out:
    - post-handle runtime rejection parity and capability-matrix regeneration (handled in `S3`).
- **Acceptance criteria**:
  - Both built-in backends advertise `agent_api.exec.add_dirs.v1` after implementation.
  - Codex exec/resume emit repeated `--add-dir <DIR>` pairs after any accepted `--model` pair and
    emit no `--add-dir` when the key is absent.
  - Claude fresh/resume/fork emit one `--add-dir <DIR...>` group after `--model`, before
    `--continue` / `--fork-session` / `--resume`, and before the final `--verbose`.
  - Codex fork selector `"last"` and selector `"id"` reject accepted add-dir inputs before any
    `thread/list`, `thread/fork`, or `turn/start` request.
  - Invalid fork + add-dir inputs fail as `InvalidRequest` before the Codex-specific backend
    rejection path.
- **Dependencies**:
  - SEAM-3 Codex mapping + fork rejection (`AD-C05`, `AD-C08`).
  - SEAM-4 Claude mapping (`AD-C06`).
  - `AD-C04` and `AD-C07` from the threading registry.
- **Verification**:
  - `cargo test -p agent_api --all-features`
- **Rollout/safety**:
  - Test-only slice. Land after SEAM-3 and SEAM-4 settle so the assertions pin final truth rather
    than interim behavior.

#### S2.T1 — Add Codex capability + exec/resume argv conformance tests

- **Outcome**: Codex test coverage pins capability publication, repeated-pair argv shape, model
  ordering, and no-flag absence behavior for accepted add-dir inputs.
- **Inputs/outputs**:
  - Input: `AD-C05` and `AD-C07`.
  - Output: Codex tests under `crates/agent_api/src/backends/codex/tests/**` that assert:
    - capabilities include `agent_api.exec.add_dirs.v1`,
    - exec and resume place every `--add-dir <DIR>` pair after any accepted `--model` pair,
    - accepted inputs preserve normalized order,
    - absent inputs emit no `--add-dir`.
- **Implementation notes**:
  - Reuse normalized-path fixtures from `S1` where possible, but keep argv assertions backend-local.
  - Keep ordering assertions exact enough to catch a `--model` / `--add-dir` inversion.
- **Acceptance criteria**:
  - A regression that reorders `--model` after `--add-dir`, drops a directory, or emits a flag when
    the key is absent fails deterministically.
- **Test notes**:
  - Run: `cargo test -p agent_api --all-features codex`.
- **Risk/rollback notes**:
  - None. Pure tests.

Checklist:
- Implement: add Codex capability, exec argv, resume argv, and absence-semantics assertions.
- Test: `cargo test -p agent_api --all-features codex`.
- Validate: confirm the expected argv order matches `AD-C05` exactly.
- Cleanup: remove any stale backend-local assertions that conflict with the shared contract.

#### S2.T2 — Add Codex fork branch coverage for rejection boundary and invalid-input precedence

- **Outcome**: Codex fork coverage proves the only accepted-input exception is the pinned safe
  backend rejection, and malformed inputs still fail earlier as `InvalidRequest`.
- **Inputs/outputs**:
  - Input: `AD-C04` and `AD-C08`.
  - Output: Codex fork tests under `crates/agent_api/src/backends/codex/tests/**` that assert:
    - selector `"last"` rejects accepted add-dir inputs before any `thread/list`, `thread/fork`,
      or `turn/start`,
    - selector `"id"` rejects accepted add-dir inputs before any `thread/list`, `thread/fork`, or
      `turn/start`,
    - malformed / missing / non-directory add-dir payloads fail as exact safe `InvalidRequest`
      messages before the fork-specific backend rejection path.
- **Implementation notes**:
  - The tests should prove both branch coverage and “no request sent” boundaries explicitly rather
    than inferring them from a generic error.
  - Keep invalid-input precedence separate from accepted-input rejection so the failure mode is
    obvious.
- **Acceptance criteria**:
  - Any attempt to send a fork RPC before the pinned rejection path fails the test.
- **Test notes**:
  - Run: `cargo test -p agent_api --all-features codex`.
- **Risk/rollback notes**:
  - None. Pure tests.

Checklist:
- Implement: add accepted-input rejection tests for selectors `"last"` and `"id"`.
- Test: add malformed-payload precedence tests for the same fork surfaces.
- Validate: assert zero app-server requests are sent on both rejection paths.
- Cleanup: keep the safe-message assertions aligned with `AD-C03` / `AD-C08`.

#### S2.T3 — Add Claude capability + selector-branch argv placement tests

- **Outcome**: Claude coverage pins one variadic `--add-dir` group across fresh-run, resume, and
  fork flows, including the branch-specific `"last"` / `"id"` argv subsequences.
- **Inputs/outputs**:
  - Input: `AD-C04`, `AD-C06`, and `AD-C07`.
  - Output: Claude tests under `crates/agent_api/src/backends/claude_code/tests/**` that assert:
    - capabilities include `agent_api.exec.add_dirs.v1`,
    - fresh-run emits one variadic `--add-dir <DIR...>` group in normalized order,
    - resume selector `"last"` and selector `"id"` each keep that group after `--model` and before
      `--continue` / `--resume`,
    - fork selector `"last"` and selector `"id"` each keep that group after `--model` and before
      `--fork-session` / `--resume`,
    - absent inputs emit no `--add-dir`,
    - the group stays before the final `--verbose`.
- **Implementation notes**:
  - Use separate tests for each selector branch because the downstream argv subsequences differ.
  - Keep the assertion focused on placement and presence, not on unrelated Claude flags.
- **Acceptance criteria**:
  - A regression that duplicates the group, places it after branch flags, or omits it for resume or
    fork fails deterministically.
- **Test notes**:
  - Run: `cargo test -p agent_api --all-features claude_code`.
- **Risk/rollback notes**:
  - None. Pure tests.

Checklist:
- Implement: add Claude capability publication coverage.
- Test: add fresh-run, resume `"last"` / `"id"`, and fork `"last"` / `"id"` argv placement tests.
- Validate: assert the final `--verbose` still comes after the `--add-dir` group.
- Cleanup: keep absence-semantics assertions local to backend mapping, not shared helper logic.
