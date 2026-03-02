# Contradictions Remediation Log — `agent-api-explicit-cancellation`

Date (UTC): 2026-02-24

## Inputs

- Contradictions report:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/contradictions-audit.report.json`

## Triage

- CX-0001 (major, medium) — *Scope mismatch / terminology drift* within ADR-0014:
  - Bucket: **A. Scope clarification** (make the meaning of “cancel” explicit) + **B. Outdated wording**
    (align ADR language to the normative run protocol spec).

## Resolutions

### CX-0001 — ADR-0014: drop semantics vs cancellation meaning (Fixed)

**Restated contradiction**

Within `docs/adr/0014-agent-api-explicit-cancellation.md`, one section read as “dropping must not imply
cancel”, while a later section stated drop semantics remain “best-effort cancellation” per the run
protocol. Without scoping what “cancel” means, these read as contradictory.

**Truth-finding (evidence-first)**

- Normative spec defines drop as a *best-effort cancellation signal* and requires explicit cancellation
  for deterministic “stop” control:
  - `docs/specs/universal-agent-api/run-protocol-spec.md:64-70`
- Backend harness design explicitly preserves drain-on-drop behavior to avoid deadlocks/cancellation hazards:
  - `docs/adr/0013-agent-api-backend-harness.md:37-45`
  - `docs/adr/0013-agent-api-backend-harness.md:61-74`

**Resolution type**

- **Scoped truth / terminology clarification**:
  - Drop remains a best-effort signal (not reliable; not deterministic).
  - Explicit cancellation is the supported “intentional cancel” mechanism.

**Doc changes applied**

- Clarified ADR-0014 text so “dropping must not imply cancel” is explicitly scoped to
  *intentional/deterministic cancellation*, while preserving that drop is a best-effort signal:
  - `docs/adr/0014-agent-api-explicit-cancellation.md:66-70`
- Updated ADR drift guard:
  - `make adr-fix ADR=docs/adr/0014-agent-api-explicit-cancellation.md`

**Decisions introduced**

- None (sufficient authoritative evidence exists in the normative run protocol spec).

## Verification

- Re-scanned the same document set:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/contradictions-audit.scan.after.json`
- Manual re-check of the original cited locations confirms CX-0001 is now scope-consistent:
  - ADR-0014 now scopes “cancel” to intentional/deterministic cancellation while preserving drop as
    a best-effort signal (`docs/adr/0014-agent-api-explicit-cancellation.md:66-70`).
  - ADR-0014 still states drop semantics remain best-effort cancellation per the run protocol
    (`docs/adr/0014-agent-api-explicit-cancellation.md:142-144`).
