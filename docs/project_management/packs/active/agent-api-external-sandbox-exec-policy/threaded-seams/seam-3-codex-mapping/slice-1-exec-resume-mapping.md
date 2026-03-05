### S1 — Exec/resume mapping (danger bypass) + pre-spawn validation + warning

- **User/system value**: makes Codex exec/resume deterministic and non-interactive in external
  sandbox mode, unblocking SEAM-5 mapping tests and preventing accidental interactive hangs.
- **Scope (in/out)**:
  - In:
    - Parse `extensions["agent_api.exec.external_sandbox.v1"]` as boolean (default `false`) and
      validate before spawn.
    - Enforce contradictions when `external_sandbox=true` (pre-spawn):
      - reject `agent_api.exec.non_interactive=false` (ES-C02),
      - reject any `backend.codex.exec.*` keys (ES-C06).
    - Emit the pinned warning `Status` event when `external_sandbox=true` is accepted (exact
      message + ordering per `docs/specs/universal-agent-api/extensions-spec.md`).
    - Apply the pinned exec/resume mapping: configure
      `codex::CodexClientBuilder::dangerously_bypass_approvals_and_sandbox(true)`.
    - Fail closed when the installed Codex binary rejects the pinned flag (no spawn+retry loop);
      surface `AgentWrapperError::Backend { message }` with a safe/redacted `message`.
  - Out:
    - Fork/app-server mapping (S2).
    - Capability advertising / opt-in gating (SEAM-2).
    - Regression tests (SEAM-5).
- **Acceptance criteria**:
  - When `external_sandbox` is absent/`false`, behavior is unchanged.
  - When `external_sandbox=true` is accepted:
    - exec/resume argv contains exactly one `--dangerously-bypass-approvals-and-sandbox`,
    - exec/resume argv contains none of: `--full-auto`, `--ask-for-approval`, `--sandbox`
      (per `docs/specs/codex-external-sandbox-mapping-contract.md`).
  - `external_sandbox=true` + `agent_api.exec.non_interactive=false` fails before spawn as
    `AgentWrapperError::InvalidRequest` (ES-C02).
  - `external_sandbox=true` + any `backend.codex.exec.*` key present fails before spawn as
    `AgentWrapperError::InvalidRequest` (ES-C06).
  - When `external_sandbox=true` is accepted, exactly one warning `Status` event is emitted with
    the pinned message, before any `TextOutput`/`ToolCall`/`ToolResult` events and before the
    session handle facet `Status` event (if `agent_api.session.handle.v1` is advertised).
  - When the Codex binary rejects the pinned dangerous bypass flag, the run fails as
    `AgentWrapperError::Backend { message }` with a safe/redacted `message` (no raw stderr), and
    no fallback mapping is attempted.
- **Dependencies**:
  - `SEAM-2` opt-in gating: the key must be supported/advertised only when enabled (ES-C03).
  - Canonical mapping contract: `docs/specs/codex-external-sandbox-mapping-contract.md` (ES-C04).
  - Key semantics + warning contract: `docs/specs/universal-agent-api/extensions-spec.md`.
- **Verification**:
  - Compile + existing tests: `cargo test -p agent_api codex`
  - SEAM-5 adds pinned tests for argv shape, contradictions, and warning ordering.
- **Rollout/safety**:
  - Safe-by-default: capability is unreachable unless the host opts in (SEAM-2) and the run
    explicitly requests the key.
  - Deterministic: no spawn+retry loops for mapping primitive mismatches.

#### S1.T1 — Add external sandbox policy extraction and contradiction validation

- **Outcome**: the Codex backend extracts `agent_api.exec.external_sandbox.v1` into the run policy
  and enforces ES-C02/ES-C06 contradictions before spawn.
- **Inputs/outputs**:
  - Input: SEAM-3 brief + `docs/specs/universal-agent-api/extensions-spec.md` validation rules.
  - Output: code changes in `crates/agent_api/src/backends/codex.rs`:
    - add `EXT_EXTERNAL_SANDBOX_V1: &str = "agent_api.exec.external_sandbox.v1"`,
    - extend `CodexExecPolicy` with `external_sandbox: bool` (default `false`),
    - parse `external_sandbox` as boolean when present,
    - when `external_sandbox=true`:
      - reject `agent_api.exec.non_interactive=false` if explicitly present,
      - reject presence of `backend.codex.exec.approval_policy` and/or
        `backend.codex.exec.sandbox_mode`.
- **Implementation notes**:
  - Apply contradiction checks only after R0 allowlisting (already enforced by the harness).
  - Prefer checking for explicit presence of `agent_api.exec.non_interactive` to distinguish
    “absent (default true)” from “explicit false”.
  - Use the existing `parse_bool` helper for type validation.
- **Acceptance criteria**:
  - Meets the slice contradiction-related acceptance criteria.
  - Validation happens in `validate_and_extract_policy(...)` (i.e., pre-spawn).
- **Test notes**:
  - SEAM-5 will pin behavior; for this task, run `cargo test -p agent_api codex` for regressions.
- **Risk/rollback notes**:
  - Low risk: only affects requests that include the new key (and only when supported via SEAM-2).

Checklist:
- Implement: policy field + parsing + contradiction checks in `crates/agent_api/src/backends/codex.rs`.
- Test: `cargo test -p agent_api codex`.
- Validate: `rg -n "agent_api\\.exec\\.external_sandbox\\.v1" crates/agent_api/src/backends/codex.rs`.
- Cleanup: keep error messages safe (no raw backend output).

#### S1.T2 — Emit the pinned “dangerous mode enabled” warning event with correct ordering

- **Outcome**: when `external_sandbox=true` is accepted, the Codex backend emits exactly one pinned
  warning `Status` event before any other user-visible output/events.
- **Inputs/outputs**:
  - Input: pinned warning requirements and ordering rules in
    `docs/specs/universal-agent-api/extensions-spec.md`.
  - Output: code changes in `crates/agent_api/src/backends/codex.rs` (and/or small helper module):
    - add a pinned warning message constant:
      `DANGEROUS: external sandbox exec policy enabled (agent_api.exec.external_sandbox.v1=true)`
    - prepend the warning to the returned backend event stream when `policy.external_sandbox == true`.
- **Implementation notes**:
  - Implement as a stream prepend in `CodexHarnessAdapter::spawn(...)` so it applies to both exec
    and fork flows and is guaranteed to occur before session handle facet emission.
  - Add a dedicated typed backend event variant (e.g., `CodexBackendEvent::ExternalSandboxWarning`)
    and map it to `mapping::status_event(Some(PINNED_WARNING.to_string()))` in
    `CodexHarnessAdapter::map_event(...)`.
  - Prepend the warning by wrapping the typed backend stream
    (`futures_util::stream::once(...).chain(events)`) so the warning is emitted even if the Codex
    process exits before producing any `ThreadEvent`s.
  - Ensure “accepted” gating: warn only when the key is supported (R0 passed) and
    `policy.external_sandbox == true` (validation passed).
- **Acceptance criteria**:
  - Exactly one warning `Status` event per run when `external_sandbox=true` is accepted.
  - Warning ordering matches `extensions-spec.md` (before `TextOutput`/`Tool*`, before session handle facet).
- **Test notes**:
  - SEAM-5 should add an ordering test that observes the warning before tool/text output and before
    the session handle facet `Status` event.
- **Risk/rollback notes**:
  - Low risk: an additive `Status` event gated behind an explicit dangerous opt-in.

Checklist:
- Implement: prepend warning event in `crates/agent_api/src/backends/codex.rs` spawn path.
- Test: `cargo test -p agent_api codex`.
- Validate: `rg -n "DANGEROUS: external sandbox exec policy enabled" crates/agent_api/src/backends/codex.rs`.
- Cleanup: ensure the warning is not emitted when the key is absent/false/unsupported/invalid.

#### S1.T3 — Apply the pinned exec/resume mapping (danger bypass override)

- **Outcome**: Codex exec/resume runs use the pinned dangerous bypass override when the key is
  accepted, matching ES-C04 exactly.
- **Inputs/outputs**:
  - Input: `docs/specs/codex-external-sandbox-mapping-contract.md` exec/resume section.
  - Output: code changes in:
    - `crates/agent_api/src/backends/codex.rs` (thread policy into exec flow request),
    - `crates/agent_api/src/backends/codex/exec.rs` (apply the builder override).
- **Implementation notes**:
  - Add `external_sandbox: bool` to `exec::ExecFlowRequest`.
  - In `spawn_exec_or_resume_flow(...)`, when `external_sandbox == true`, configure the client
    builder with `dangerously_bypass_approvals_and_sandbox(true)` before `build()`.
  - Do not attempt any fallback mapping (no spawn+retry loop).
- **Acceptance criteria**:
  - Meets the exec/resume mapping acceptance criteria (argv contains the pinned flag and excludes
    other safety flags).
- **Test notes**:
  - SEAM-5 adds argv-shape tests; for this task, rely on compile + existing codex backend tests.
- **Risk/rollback notes**:
  - Medium risk: changes a dangerous surface; mitigate by keeping the mapping pinned and covered
    by SEAM-5 tests as soon as possible.

Checklist:
- Implement: pass `external_sandbox` into `ExecFlowRequest` and apply the builder override in `exec.rs`.
- Test: `cargo test -p agent_api codex`.
- Validate: `rg -n "dangerously_bypass_approvals_and_sandbox" crates/agent_api/src/backends/codex/exec.rs`.
- Cleanup: ensure other exec-policy flags are not introduced alongside the pinned override.

#### S1.T4 — Fail closed when the Codex binary rejects the pinned dangerous bypass flag

- **Outcome**: if the installed Codex binary does not support the pinned bypass flag, the run fails
  as `AgentWrapperError::Backend { message }` with a safe/redacted `message` (no raw stderr), and no
  fallback mapping is attempted.
- **Inputs/outputs**:
  - Input: `docs/specs/codex-external-sandbox-mapping-contract.md` “Unavailable mapping primitive behavior”.
  - Output: code changes in `crates/agent_api/src/backends/codex/exec.rs` completion/error handling.
- **Implementation notes**:
  - Detect the “unknown flag” class of failures only when `external_sandbox == true` (e.g., stderr
    contains the pinned flag name and an “unknown option/flag” signal).
  - Convert that failure into an `AgentWrapperError::Backend` outcome by:
    - emitting a terminal `Error` event with a safe/redacted message, and
    - setting `selection_failure_message` (or an equivalent pinned backend-error path) so
      `map_completion(...)` returns `AgentWrapperError::Backend`.
  - Do not include raw stderr in the surfaced message.
- **Acceptance criteria**:
  - Unsupported flag scenario yields a backend error (not just a non-zero exit status completion).
  - The final `message` is safe/redacted and stable enough to pin in SEAM-5 tests.
- **Test notes**:
  - SEAM-5 can unit-test this by stubbing the codex exec layer to return a `NonZeroExit` with an
    “unknown flag” stderr and asserting backend-error mapping + warning ordering.
- **Risk/rollback notes**:
  - Low risk: only affects external sandbox mode error translation, and improves safety.

Checklist:
- Implement: error classification + backend-error mapping in `crates/agent_api/src/backends/codex/exec.rs`.
- Test: `cargo test -p agent_api codex`.
- Validate: ensure no raw stderr is surfaced (`rg -n "stderr" crates/agent_api/src/backends/codex/exec.rs` and review formatting paths).
- Cleanup: keep the detection logic narrow to avoid misclassifying normal non-zero exits.
