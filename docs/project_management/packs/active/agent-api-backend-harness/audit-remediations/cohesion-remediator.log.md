# Cohesion Remediator Log — `agent-api-backend-harness` pack

Generated (local): 2026-02-23

Input report: `docs/project_management/packs/active/agent-api-backend-harness/cohesion-audit.report.json`

Scope (from report meta): `docs/project_management/packs/active/agent-api-backend-harness/**`

## Triage

- `CH-0002` (critical): SEAM-3/SEAM-4 handshake ambiguity → **Sequence + seam continuity repair**.
- `CH-0001` (major): `BH-C03` contract drift (env-only vs env+timeout) → **Cross-doc canonicalization**.
- `CH-0003` (major): “invalid request” checks planned without traceability despite “no behavior change” → **Traceability repair (docs + code evidence)**.
- `CH-0004` (minor): Harness test placement left TBD → **Canonicalization**.
- `CH-0005` (minor): Key symbol names left TBD → **Definition/naming closure**.

## CH-0001 — Fixed

### Cohesion break (restated)
`threading.md` is treated as the canonical contract registry, but `BH-C03` was defined as env precedence only while downstream SEAM-2/SEAM-5 slices treat `BH-C03` as covering env + timeout semantics.

### Remediation pattern
**Canonicalize**: expand the canonical contract registry entry for `BH-C03` to match the already-planned scope (env + timeout), and align dependent references.

### Evidence used
- Pack intent: SEAM-2 scope explicitly includes timeout derivation/wrapping as a shared invariant (`seam-2-request-normalization.md`).
- Existing backend behavior already derives env + timeout in both adapters (`crates/agent_api/src/backends/codex.rs`, `crates/agent_api/src/backends/claude_code.rs`).

### Doc changes applied
- Canonicalized `BH-C03` name + definition to include timeout semantics:
  - `docs/project_management/packs/active/agent-api-backend-harness/threading.md`
- Aligned contract references in threaded seam docs:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/seam.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-5-backend-adoption-and-tests/seam.md`
- Aligned SEAM-2 slice wording to match canonical contract name/definition:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md`

### Decisions introduced
- None (aligned the registry with already-stated seam scope and existing behavior).

## CH-0002 — Fixed

### Cohesion break (restated)
SEAM-3 described a single pump that drains events while also polling the backend completion future, while SEAM-4’s canonical handle builder described a split lifecycle (pump/drainer + separate completion sender). Without an explicit handshake, implementers could build incompatible designs (double-driving completion futures, mismatched finality signaling).

### Remediation pattern
**Seam continuity repair**: establish one canonical internal lifecycle handshake and align SEAM-3 + SEAM-4 docs/slices to it.

### Evidence used
- Canonical builder intent: SEAM-4 Slice S2 already documents an explicit lifecycle split (pump/drainer vs completion sender).
- Executable gating truth: `run_handle_gate` gates completion observability on sender-drop finality or consumer drop (`crates/agent_api/src/run_handle_gate.rs`).
- Prior doc remediation context: `contradictions-remediator.log.md` already pinned a split-lifecycle description for `BH-C05` to satisfy consumer-drop semantics.

### Doc changes applied
- Added a canonical SEAM-3 ↔ SEAM-4 handshake (responsibility split + single-poll rule) to the pack’s threading backbone:
  - `docs/project_management/packs/active/agent-api-backend-harness/threading.md`
- Aligned SEAM-3 seam brief + threaded decomposition + slices to describe the pump/drainer as **stream-only** (finality signaling + drain-on-drop), with completion future owned by SEAM-4:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/seam.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-1-bh-c04-drain-while-polling-completion.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-2-bh-c04-drain-on-drop-semantics.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-3-streaming-pump-unit-tests.md`
- Aligned SEAM-4 seam brief + threaded decomposition to treat SEAM-3 as defining the **finality signal** (sender drop), not “completion eligibility”:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-4-completion-gating.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/seam.md`
  - Minor terminology alignment in SEAM-4 slices:
    - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-1-bh-c05-gating-semantics.md`
    - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-2-bh-c05-canonical-handle-builder.md`

### Decisions introduced
- None (picked the already-documented split lifecycle as canonical and removed the conflicting SEAM-3 wording).

## CH-0003 — Fixed

### Cohesion break (restated)
The pack planned to implement + unit-test a “universal invalid request check” (empty prompt) while also claiming “no behavior change”, but it lacked a traceable source of truth showing this rule is already enforced across built-in backends.

### Remediation pattern
**Traceability repair**: add explicit code evidence that the empty-prompt rule already exists, and scope the harness check as behavior-preserving.

### Evidence used
- Existing behavior: both built-in adapters already reject `request.prompt.trim().is_empty()` with `InvalidRequest { message: \"prompt must not be empty\" }`:
  - `crates/agent_api/src/backends/codex.rs`
  - `crates/agent_api/src/backends/claude_code.rs`

### Doc changes applied
- Added explicit “existing behavior” evidence notes in SEAM-2 and its slices:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-1-bh-c02-extension-allowlist-validator.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-3-normalization-unit-tests.md`
- Reinforced “no behavior change” intent in adoption seam by tying the invalid-request rule to current behavior:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-5-backend-adoption-and-tests.md`

### Decisions introduced
- None (traceability added; no behavior scope change).

## CH-0004 — Fixed

### Cohesion break (restated)
SEAM-5 left harness test placement as TBD even though upstream seams describe harness-owned tests as a concrete ownership surface.

### Remediation pattern
**Canonicalize**: pick and document one test placement rule, then reference it consistently.

### Evidence used
- Existing repo pattern already uses `crates/agent_api/tests/*` for DR/protocol regression tests (e.g., `dr0012_completion_gating.rs`).

### Doc changes applied
- Added canonical test placement rules to the pack backbone:
  - `docs/project_management/packs/active/agent-api-backend-harness/threading.md`
- Removed TBD wording in SEAM-5 touch-surface and aligned to the rule:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-5-backend-adoption-and-tests.md`

### Decisions introduced
- None (aligned docs to existing test placement conventions).

## CH-0005 — Fixed

### Cohesion break (restated)
Several atomic tasks left key symbol names as TBD, weakening searchability and cross-slice traceability.

### Remediation pattern
**Definition closure**: pin concrete symbol names consistent with existing naming conventions used in current backend adapters.

### Evidence used
- Existing code naming convention for request parsing/validation helpers: `validate_and_extract_*` (e.g., `validate_and_extract_exec_policy`, `validate_and_extract_non_interactive`).

### Doc changes applied
- Pinned `BH-C01` hook names:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`
- Pinned `BH-C02` validator helper name:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-1-bh-c02-extension-allowlist-validator.md`
- Pinned `BH-C03` helper names for env + timeout:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md`

### Decisions introduced
- None (names chosen to match existing internal helper naming style and to improve traceability).

