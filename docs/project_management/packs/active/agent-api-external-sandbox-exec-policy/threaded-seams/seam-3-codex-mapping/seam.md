# SEAM-3 — Codex backend mapping (threaded decomposition)

> Pack: `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/`
> Seam brief: `seam-3-codex-mapping.md`
> Threading source of truth: `threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-3
- **Name**: Codex mapping for `agent_api.exec.external_sandbox.v1`
- **Goal / value**: when enabled + requested, run Codex in a mode compatible with external
  sandboxing by relaxing internal approvals/sandbox guardrails without prompting.
- **Type**: capability (backend mapping)
- **Scope**
  - In:
    - Validate `extensions["agent_api.exec.external_sandbox.v1"]` as boolean before spawn (when the
      capability is enabled).
    - Enforce contradiction rules (pre-spawn; fail-closed):
      - `external_sandbox=true` + `agent_api.exec.non_interactive=false` → `AgentWrapperError::InvalidRequest`
      - `external_sandbox=true` + any `backend.codex.exec.*` key present → `AgentWrapperError::InvalidRequest`
    - Emit the pinned warning `Status` event when `external_sandbox=true` is accepted (exact
      message + ordering per `docs/specs/universal-agent-api/extensions-spec.md`).
    - Apply the pinned mapping contract (ES-C04) across all Codex entrypoints:
      - Exec/resume: configure
        `codex::CodexClientBuilder::dangerously_bypass_approvals_and_sandbox(true)`.
      - Fork/app-server: send `approvalPolicy="never"` + `sandbox="danger-full-access"` on
        `thread/fork`, and `approvalPolicy="never"` on `turn/start`.
    - Fail closed (no fallback mapping) when the pinned mapping primitive is rejected (flag or
      JSON-RPC params), surfacing `AgentWrapperError::Backend { message }` with a safe/redacted
      `message`.
  - Out:
    - Host opt-in + capability advertising posture (`allow_external_sandbox_exec`) (SEAM-2).
    - Universal key semantics / R0 precedence / warning event contract text (SEAM-1).
    - Regression test coverage for the final behavior (SEAM-5).
- **Touch surface**:
  - `crates/agent_api/src/backends/codex.rs`
  - `crates/agent_api/src/backends/codex/exec.rs`
  - `crates/agent_api/src/backends/codex/fork.rs`
- **Verification**:
  - Local compile + existing Codex backend tests: `cargo test -p agent_api codex`
  - SEAM-5 adds pinned mapping/validation tests once this code lands.
- **Threading constraints**
  - Upstream blockers: SEAM-1 (ES-C01/02/06) + SEAM-2 (ES-C03)
  - Downstream blocked seams: SEAM-5
  - Contracts produced (owned): ES-C04
  - Contracts consumed: ES-C01, ES-C02, ES-C03, ES-C06

Implementation note: `docs/specs/codex-external-sandbox-mapping-contract.md` is the canonical,
Normative source of truth for ES-C04. Treat the slices below as a code conformance checklist.

## Slice index

- `S1` → `slice-1-exec-resume-mapping.md`: validate + warn + map external sandbox mode for Codex
  exec/resume flows.
- `S2` → `slice-2-fork-app-server-mapping.md`: apply pinned external sandbox params for Codex
  app-server fork flows.

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `ES-C04`: Codex mapping contract — when enabled + requested, Codex mapping is pinned (exec/resume
    uses `dangerously_bypass_approvals_and_sandbox(true)`; fork/app-server uses
    `approvalPolicy="never"` + `sandbox="danger-full-access"`). Canonical:
    `docs/specs/codex-external-sandbox-mapping-contract.md`.
    - Produced by: `S1` (exec/resume conformance) + `S2` (fork/app-server conformance).
- **Contracts consumed**:
  - `ES-C01`: External sandbox execution policy extension key — the key id
    `agent_api.exec.external_sandbox.v1` (boolean; validated before spawn) owned by SEAM-1
    (`docs/specs/universal-agent-api/extensions-spec.md`).
    - Consumed by: `S1.T1` (value validation + extraction) and `S1.T2` (warning event gating).
  - `ES-C02`: Non-interactive invariant — `external_sandbox=true` MUST NOT be combined with
    `agent_api.exec.non_interactive=false` (InvalidRequest) owned by SEAM-1.
    - Consumed by: `S1.T1` (contradiction validation).
  - `ES-C03`: Safe default advertising — Codex MUST NOT advertise the key by default; host opt-in
    via backend config `allow_external_sandbox_exec` (canonical: `docs/specs/universal-agent-api/contract.md`).
    - Consumed by: `S1`/`S2` (assumption: runs only reach mapping when opt-in is enabled).
  - `ES-C06`: Exec-policy combination rule — forbid any `backend.<agent_kind>.exec.*` keys when
    `external_sandbox=true` (InvalidRequest) owned by SEAM-1.
    - Consumed by: `S1.T1` (reject `backend.codex.exec.*` keys when `external_sandbox=true`).
- **Dependency edges honored**:
  - `SEAM-2 blocks SEAM-3`: this plan assumes Codex only supports the key behind explicit host
    opt-in (ES-C03); mapping work is unreachable until SEAM-2 exposes the capability id and
    allowlists the key under R0.
  - `SEAM-3 blocks SEAM-5`: `S1`/`S2` provide the final Codex mapping behavior that SEAM-5 pins in
    tests.
- **Parallelization notes**:
  - What can proceed now: prepare SEAM-3 implementation details in `codex/exec.rs` and `codex/fork.rs`
    (minimal overlap), but land after SEAM-2 to avoid merge conflicts in `codex.rs`.
  - What must wait: `S1`/`S2` require SEAM-2’s opt-in enablement to make the key supported; SEAM-5
    must wait for `S1`/`S2` to finalize mapping behavior.

