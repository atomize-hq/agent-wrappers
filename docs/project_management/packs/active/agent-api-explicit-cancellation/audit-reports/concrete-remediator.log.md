# Concrete Remediation Log — `agent-api-explicit-cancellation`

Date (UTC): 2026-02-24

Input audit report:
- `docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/concrete-audit.report.json`

Scope:
- Docs-only remediation (no code changes).
- Goal: close all issue IDs in the report by making the referenced docs fully concrete.

## Triage

Issues by severity:
- Critical: CA-0001
- Major: CA-0002, CA-0003, CA-0006
- Minor: CA-0004, CA-0005

Buckets (fix strategy):
- A. Local clarification: CA-0004, CA-0005
- B. Code-defined contract (evidence source): CA-0003 (matrix generator semantics), CA-0005 (actual module layout), CA-0002 (CI platform)
- C. Doc-defined standard: CA-0001, CA-0002, CA-0006
- D. External standard: (none used)
- E. Decision required: (none)

## Files changed

- `docs/adr/0013-agent-api-backend-harness.md`
- `docs/adr/0014-agent-api-explicit-cancellation.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/decision_register.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md`
- `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md`
- `docs/specs/universal-agent-api/capabilities-schema-spec.md`
- `docs/specs/universal-agent-api/run-protocol-spec.md`

## Issue-by-issue remediation

### CA-0001 — Explicit cancellation completion precedence + gating are ambiguous/inconsistent in ADR-0014 and SEAM-2 docs

Status: **Fixed**

Restated requirement:
- Pin cancellation precedence and tie-breaking in `Result` terms (cancellation overrides any not-yet-resolved `Ok(...)` or `Err(...)` and wins concurrent readiness), and restate that completion *value* selection does not relax DR-0012 completion *timing*.

Evidence used:
- Canonical explicit cancellation semantics + DR-0012 completion gating:
  - `docs/specs/universal-agent-api/run-protocol-spec.md` L64-L118
- SEAM-2 driver model and slice acceptance criteria are the pinned pack-level driver contract:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md` L41-L59
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md` L1-L35

Changes made:
- Updated `docs/adr/0014-agent-api-explicit-cancellation.md` to:
  - replace “before success completion” phrasing with `Ok(...)`/`Err(...)`-based precedence,
  - pin tie-breaking (“cancellation wins”) and explicitly state cancellation overrides backend error completion when requested first,
  - restate DR-0012 gating explicitly (value selection ≠ timing), and
  - explicitly point implementers/consumers to runtime capability checks via `AgentWrapperCapabilities.ids` (matrix is non-exhaustive).
- Updated `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md` and the threaded SEAM-2 driver slice to:
  - restate precedence using `Ok(...)`/`Err(...)` terms,
  - pin tie-breaking (concurrent readiness), and
  - keep explicit cross-links to run protocol gating + event-stream closure rules.

Decisions introduced:
- None (aligned to the already-normative run protocol spec).

### CA-0002 — SEAM-4 test contract does not pin a non-ambiguous termination signal or clearly connect seam-level requirements to the pinned slice parameters

Status: **Fixed**

Restated requirement:
- Pin a cross-platform, non-ambiguous termination signal for the SEAM-4 explicit-cancel integration test and make the inference chain concrete (stream closure is required by cancellation and is not itself a termination signal).
- Define “supported platforms” for the pinned timeouts so “same timeouts on all supported platforms” is enforceable.

Evidence used:
- DR-0012 defines completion gating in terms of backend process exit + stream finality, making `completion` resolution a usable termination signal for this test contract:
  - `docs/specs/universal-agent-api/run-protocol-spec.md` L35-L49
- CI currently runs on `ubuntu-latest`:
  - `.github/workflows/ci.yml` L18-L22
- Existing pinned timeouts already live in the threaded SEAM-4 slice docs:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md` L26-L38

Changes made:
- Updated `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md` to:
  - define termination pass/fail criteria concretely:
    - `completion` resolving within `CANCEL_TERMINATION_TIMEOUT` is the termination signal (due to DR-0012 gating),
    - `events` reaching `None` is necessary to test the stream-closure rule but is not sufficient as a termination signal,
  - declare the supported platform set for v1 timeouts as CI’s `ubuntu-latest` (`x86_64-unknown-linux-gnu`),
  - explicitly declare the threaded slice docs as the authoritative source for pinned parameters + test-specific criteria.
- Updated the threaded SEAM-4 slice docs to match the pinned termination-signal definition and to reference SEAM-4’s supported-platform definition:
  - `threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md`
  - `threaded-seams/seam-4-tests/slice-2-drop-regression.md`
- Updated `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md` to reference the now-pinned supported-platform definition in SEAM-4.

Decisions introduced:
- None (supported platforms defined by existing CI configuration).

### CA-0003 — Capability matrix does not define absence semantics for standard capability ids (explicit cancellation capability missing)

Status: **Fixed**

Restated requirement:
- Define what it means for a standard capability id (e.g., `agent_api.control.cancel.v1`) to be absent from the generated capability matrix, and ensure readers treat runtime capability checks (`AgentWrapperCapabilities.ids`) as the authoritative signal.

Evidence used:
- Capability matrix generator includes only capability ids advertised by at least one built-in backend (union across built-in backends):
  - `crates/xtask/src/capability_matrix.rs` L26-L43, L53-L60
- Standard capability ids (including explicit cancellation) are defined canonically in:
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md` (standard ids registry)

Changes made:
- Updated `docs/specs/universal-agent-api/capabilities-schema-spec.md` to add a pinned “Capability matrix (generated artifact)” section that defines:
  - matrix non-exhaustiveness,
  - absence semantics (“absent => no built-in backend currently advertises it”), and
  - the requirement to use `AgentWrapperCapabilities.ids` for runtime availability checks.
- Updated `docs/adr/0014-agent-api-explicit-cancellation.md` to explicitly direct runtime availability checks to `AgentWrapperCapabilities.ids` and to note the matrix is non-exhaustive.

Decisions introduced:
- None (semantics aligned to the generator behavior already in the repo).

### CA-0006 — `run-protocol-spec.md` uses non-testable qualifiers for capability validation timing and error-event emission

Status: **Fixed**

Restated requirement:
- Replace “where possible” / “if feasible” language with concrete, testable rules for:
  - when capability validation MUST occur relative to spawning backend work, and
  - when an `AgentWrapperEventKind::Error` event MUST be emitted (and what to do when emission is not possible).

Evidence used:
- The run protocol spec is normative and already pins pre-spawn validation for extensions:
  - `docs/specs/universal-agent-api/run-protocol-spec.md` (capability validation timing section)

Changes made:
- Updated `docs/specs/universal-agent-api/run-protocol-spec.md` to define concrete rules:
  - required capability validation occurs before spawning any backend process (no “where possible”),
  - extension key/value validation occurs before spawn, and
  - post-spawn “unsupported operation” faults require exactly one `Error` event if (and only if) the consumer-visible
    stream is still open; otherwise the error is reported only via `completion`.

Decisions introduced:
- None.

### CA-0004 — DR-CA-0002 uses a `<pinned string>` placeholder instead of the pinned cancellation message

Status: **Fixed**

Restated requirement:
- Replace `<pinned string>` with the exact pinned message (`"cancelled"`) and add an explicit pointer to the canonical pinned definition so the DR cannot drift.

Evidence used:
- Pinned cancellation message is defined in:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` L70-L85
  - `docs/specs/universal-agent-api/run-protocol-spec.md` L104-L118

Changes made:
- Updated `docs/project_management/packs/active/agent-api-explicit-cancellation/decision_register.md` DR-CA-0002 to:
  - use `"cancelled"` explicitly in Option A, and
  - add explicit pointers to the canonical pinned definitions.

Decisions introduced:
- None.

### CA-0005 — ADR-0013 leaves the backend harness module name as TBD

Status: **Fixed**

Restated requirement:
- Pin the backend harness module name and canonical file path so there is exactly one stable internal module boundary in ADR-0013, and ensure the file/module boundaries section is consistent with that pin.

Evidence used:
- Backend harness module exists as a module directory (module root + submodules) in `crates/agent_api/src/backend_harness/`.

Changes made:
- Updated `docs/adr/0013-agent-api-backend-harness.md` to:
  - pin the module name as `agent_api::backend_harness`, and
  - pin the module root file as `crates/agent_api/src/backend_harness/mod.rs`,
  - update “File/module boundaries” to match the current module layout (`mod.rs` + `runtime.rs` / `normalize.rs` / `contract.rs`).

Decisions introduced:
- None.

## Verification

Completed:
- Concrete scan:
  - Before: `docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/concrete-audit.scan.json` (match_count: 38)
  - After: `docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/concrete-audit.scan.after.json` (match_count: 27)
- Manual closure check:
  - CA-0001..CA-0006 “Required to be concrete” checklists are directly addressed by the patched sections cited above.
