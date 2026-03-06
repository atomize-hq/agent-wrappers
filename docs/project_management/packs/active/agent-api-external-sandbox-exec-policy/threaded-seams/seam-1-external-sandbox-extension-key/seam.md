# SEAM-1 — Core extension key contract (threaded decomposition)

> Pack: `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/`
> Seam brief: `seam-1-external-sandbox-extension-key.md`
> Threading source of truth: `threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-1
- **Name**: `agent_api.exec.external_sandbox.v1` (external sandbox execution policy; dangerous)
- **Goal / value**: allow externally sandboxed hosts to explicitly request that built-in backends
  relax internal approvals/sandbox/permissions guardrails while remaining non-interactive.
- **Type**: risk + integration
- **Scope**
  - In:
    - Define the core extension key contract in `docs/specs/universal-agent-api/extensions-spec.md`:
      - schema (boolean) and default/absence semantics,
      - validation timing (before spawn) + R0 precedence alignment,
      - contradiction rules with `agent_api.exec.non_interactive` and `backend.<agent_kind>.exec.*`,
      - observability warning event requirement and emission ordering,
      - backend mapping requirements and canonical mapping doc references.
  - Out:
    - Backend enablement / capability advertising posture (SEAM-2).
    - Backend-specific CLI flag mapping details (SEAM-3, SEAM-4).
    - Regression tests (SEAM-5).
- **Touch surface**: `docs/specs/universal-agent-api/extensions-spec.md`
- **Verification**:
  - Spec review: confirm schema/defaults and contradiction rules are unambiguous.
  - Cross-doc check: referenced mapping contracts and opt-in contract section exist and are correctly
    named.
- **Threading constraints**
  - Upstream blockers: none
  - Downstream blocked seams: SEAM-2 (direct), SEAM-3/4/5 (transitive via SEAM-2)
  - Contracts produced (owned): ES-C01, ES-C02, ES-C06
  - Contracts consumed: none (relies on R0 gating and `agent_api.exec.non_interactive` defined in
    the same owner doc)

Implementation note: `docs/specs/universal-agent-api/extensions-spec.md` already contains a
normative entry for `agent_api.exec.external_sandbox.v1`. Treat the slices below as a conformance
checklist unless the spec needs edits.

## Slice index

- `S1` → `slice-1-contract-and-validation.md`: publish key semantics + pre-spawn validation and
  contradiction rules.
- `S2` → `slice-2-observability-and-mapping-requirements.md`: pin warning event ordering + mapping
  requirements and references.

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `ES-C01`: External sandbox execution policy extension key — defined as
    `agent_api.exec.external_sandbox.v1` (boolean; validated before spawn) in
    `docs/specs/universal-agent-api/extensions-spec.md`.
    - Produced by: `S1` (schema/default/meaning) + `S2` (mapping requirements + warning event).
  - `ES-C02`: Non-interactive invariant (external sandbox mode) — contradiction rule with
    `agent_api.exec.non_interactive=false` (fail before spawn with `AgentWrapperError::InvalidRequest`).
    - Produced by: `S1`.
  - `ES-C06`: Exec-policy combination rule (external sandbox mode) — forbid any
    `backend.<agent_kind>.exec.*` keys when `external_sandbox=true` (after R0 support gating).
    - Produced by: `S1`.
- **Contracts consumed**:
  - None.
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-2`: `S1`/`S2` pin the final key semantics required for safe capability
    advertising and backend opt-in enablement.
- **Parallelization notes**:
  - What can proceed now: `WS-SPEC` (this seam) is docs-only and conflict-safe.
  - What must wait: `SEAM-2` should wait for `S1` (stable semantics + contradiction rules) before
    implementing capability advertising/opt-in; `SEAM-3/4/5` follow the dependency graph in
    `threading.md`.

