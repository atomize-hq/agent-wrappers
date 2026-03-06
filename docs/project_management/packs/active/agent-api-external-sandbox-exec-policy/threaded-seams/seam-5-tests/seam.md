# SEAM-5 — Tests (threaded decomposition)

> Pack: `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/`
> Seam brief: `seam-5-tests.md`
> Threading source of truth: `threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-5
- **Name**: regression coverage for `agent_api.exec.external_sandbox.v1`
- **Goal / value**: prevent regressions that accidentally advertise or accept the dangerous key by
  default, or that allow interactive hangs/unsafe spawn behavior; pin the cross-backend mapping
  contracts so externally sandboxed hosts can rely on deterministic behavior.
- **Type**: integration (contract conformance) + regression
- **Scope**
  - In:
    - Capability advertising tests (default off; opt-in on) (ES-C03).
    - Harness ordering tests:
      - unsupported extension keys fail closed (R0) before any value/contradiction validation.
    - Backend validation tests (pre-spawn; fail-closed):
      - boolean type validation for `agent_api.exec.external_sandbox.v1` (ES-C01),
      - contradiction handling with `agent_api.exec.non_interactive` (ES-C02),
      - exec-policy combination rule: `external_sandbox=true` rejects any
        `backend.<agent_kind>.exec.*` keys (ES-C06),
      - no spawn when invalid / contradictory.
    - Mapping tests (required; pinned):
      - Codex (exec + resume): argv MUST contain `--dangerously-bypass-approvals-and-sandbox` and
        MUST NOT contain any of: `--full-auto`, `--ask-for-approval`, `--sandbox` (ES-C04).
      - Codex (exec + resume, rejected override): if the installed Codex binary rejects
        `--dangerously-bypass-approvals-and-sandbox`, the backend returns
        `AgentWrapperError::Backend { .. }` with a safe/redacted message and performs no fallback
        retry (ES-C04).
      - Codex (fork/app-server): RPC MUST use `approvalPolicy="never"` and
        `sandbox="danger-full-access"` on `thread/fork`, and `approvalPolicy="never"` on
        `turn/start` (ES-C04).
      - Codex (fork/app-server, rejected mapping primitive): if the app-server rejects the pinned
        `approvalPolicy` / `sandbox` values, the backend returns `AgentWrapperError::Backend { .. }`
        with a safe/redacted message, performs no fallback retry, and emits the pinned terminal
        error event when the stream remains open (ES-C04).
      - Claude Code: argv MUST contain `--dangerously-skip-permissions`, and MUST include/exclude
        `--allow-dangerously-skip-permissions` exactly per the pinned help-preflight strategy
        (ES-C05/ES-C07).
    - Warning event conformance (pinned):
      - exactly one `Status` warning with the pinned message when `external_sandbox=true` is
        accepted, and
      - warning ordering before any other user-visible events and before the session handle facet
        `Status` event.
  - Out:
    - Live-binary e2e tests in default CI lanes (explicitly gated; see `seam-5-tests.md`).
- **Touch surface**:
  - Harness: `crates/agent_api/src/backend_harness/normalize/tests.rs`
  - Backend unit tests: `crates/agent_api/src/backends/codex/tests.rs`,
    `crates/agent_api/src/backends/claude_code/tests.rs`
  - Integration tests + fakes (recommended for pinned argv/RPC assertions):
    - `crates/agent_api/tests/**`
    - `crates/agent_api/src/bin/fake_codex_*_agent_api.rs`
    - `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`
- **Verification**:
  - Targeted runs while iterating:
    - `cargo test -p agent_api backend_harness::normalize`
    - `cargo test -p agent_api codex`
    - `cargo test -p agent_api claude_code`
- **Threading constraints**
  - Upstream blockers: SEAM-1 (ES-C01/02/06 + warning contract), SEAM-2 (ES-C03), SEAM-3 (ES-C04),
    SEAM-4 (ES-C05/07)
  - Downstream blocked seams: none (end of critical path)
  - Contracts produced (owned): none (SEAM-5 pins conformance via tests)
  - Contracts consumed: ES-C01, ES-C02, ES-C03, ES-C04, ES-C05, ES-C06, ES-C07

## Slice index

- `S1` → `slice-1-advertising-r0-gating.md`: assert safe default advertising and R0 fail-closed
  ordering for the dangerous key (no value validation when unsupported).
- `S2` → `slice-2-validation-contradictions.md`: pin backend validation + contradiction behavior
  (type checks, ES-C02/ES-C06), proving failure before spawn.
- `S3` → `slice-3-mapping-conformance.md`: pin the backend-owned mapping contracts (Codex argv + RPC,
  Codex rejection-path fail-closed behavior, Claude argv + allow-flag preflight, and the required
  warning event ordering.

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - None. SEAM-5 only adds regression tests that pin conformance to upstream contract owners.
- **Contracts consumed**:
  - `ES-C01` (SEAM-1): key id + boolean schema + validated-before-spawn requirement.
    - Consumed by: `S2` (type validation tests) + `S3` (mapping gated on accepted boolean).
  - `ES-C02` (SEAM-1): `external_sandbox=true` contradicts `agent_api.exec.non_interactive=false`.
    - Consumed by: `S2` (contradiction tests for both backends).
  - `ES-C03` (SEAM-2): safe default advertising + opt-in gate (`allow_external_sandbox_exec`).
    - Consumed by: `S1` (capabilities/advertising tests) + `S1` (unsupported-key ordering tests).
  - `ES-C04` (SEAM-3): Codex mapping contract (exec/resume argv; fork/app-server RPC).
    - Consumed by: `S3.T1` (exec/resume argv exactness + rejected-override fail-closed tests) +
      `S3.T2` (fork/app-server param exactness + rejected-mapping fail-closed tests).
  - `ES-C05` (SEAM-4): Claude mapping contract (`--dangerously-skip-permissions`).
    - Consumed by: `S3.T3` (argv tests).
  - `ES-C06` (SEAM-1): exec-policy combination rule (`backend.<agent_kind>.exec.*` keys forbidden in
    external sandbox mode).
    - Consumed by: `S2` (combination-rule validation tests) and `S1` (R0 precedence when a key is
      unsupported).
  - `ES-C07` (SEAM-4): deterministic Claude allow-flag preflight (cached; fail before spawn on
    preflight failure).
    - Consumed by: `S3.T3` (allow-flag included/excluded + preflight failure tests).
- **Dependency edges honored**:
  - `SEAM-3 blocks SEAM-5`: `S3.T1`/`S3.T2` assume final Codex mapping behavior exists.
  - `SEAM-4 blocks SEAM-5`: `S3.T3` assumes final Claude mapping + preflight behavior exists.
- **Parallelization notes**:
  - What can proceed now: `S1` can land once SEAM-1/SEAM-2 are stable.
  - What must wait: `S3` mapping tests must wait for SEAM-3/SEAM-4 implementations; `S2` validation
    tests that require the key to be supported depend on SEAM-2 opt-in gating.
