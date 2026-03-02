# Cohesion Remediation Log — `agent-api-explicit-cancellation`

Date (UTC): 2026-02-24

## Inputs

- Cohesion report:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/cohesion-audit.report.json`

## Triage

- CH-0002 (critical) — **missing bridge** between cancellation “wins” and DR-0012 completion gating  
  - Bucket: **D. Seam continuity repair** + **C. Sequence repair**
- CH-0001 (major) — **scope disconnect**: Approved specs vs Draft ADR/pack; rollout/support unclear  
  - Bucket: **E. Traceability repair**
- CH-0003 (major) — **missing definitions**: BH-C04/BH-C05 shorthand is used without canonical links  
  - Bucket: **A. Local reflow** + **F. Reference integrity**
- CH-0004 (major) — **canonicalization**: pinned timeouts live in threaded slices but are not surfaced at seam output level  
  - Bucket: **B. Cross-doc canonicalization** + **F. Reference integrity**
- CH-0005 (major) — **verification gap**: cancel-handle lifetime rule is not traced to SEAM-4 coverage  
  - Bucket: **E. Traceability repair**
- CH-0006 (major) — **canonicalization**: ADRs cite `docs/project_management/next/universal-agent-api/*` as “authoritative”  
  - Bucket: **B. Cross-doc canonicalization** + **F. Reference integrity**

## Resolutions

### CH-0002 — Cancellation completion outcome selection is not clearly bridged to DR-0012 completion gating (Fixed)

**Restated cohesion break**

Execution docs discussed “cancel wins” and the pinned `"cancelled"` completion outcome without
explicitly bridging to the pinned DR-0012 completion gating rule (timing), making it plausible to
misread “cancel wins” as allowing completion to resolve before backend process exit.

**Evidence (authoritative)**

- DR-0012 completion gating and consumer opt-out semantics:
  - `docs/specs/universal-agent-api/run-protocol-spec.md:35-49`
- Explicit cancellation completion gating is explicitly pinned (cancellation does not accelerate completion timing):
  - `docs/specs/universal-agent-api/run-protocol-spec.md:113-118`

**Remediation pattern**

- **Bridge + qualify**: treat “cancel wins” as *completion value selection* and explicitly restate that
  completion *timing* remains gated by DR-0012.

**Doc changes applied**

- Added an explicit “completion timing / gating” bridge to SEAM-1 contract:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md`
- Added DR-0012 gating language to SEAM-2 driver model and requirements (value vs timing):
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md`
- Qualified “cancel wins” language in threaded SEAM-2 docs to prevent “early completion” interpretations:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/seam.md`
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md`
- Added explicit DR-0012 gating acceptance criteria under cancellation in SEAM-4 test slice:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md`

**Decisions introduced**

- None (fully grounded in the pinned run protocol spec).

---

### CH-0001 — Explicit cancellation appears as both Draft (ADR/pack) and Approved (canonical specs); current support/rollout state is unclear (Fixed)

**Restated cohesion break**

Readers could not tell whether explicit cancellation was “already shipped” (Approved specs) vs “still
planned” (Draft ADR/pack), and the capability matrix did not show any `agent_api.control.*` section.

**Evidence**

- Canonical specs are Approved and define explicit cancellation + DR-0012 gating:
  - `docs/specs/universal-agent-api/run-protocol-spec.md:72-118`
- The generated capability matrix reflects current backend-advertised capabilities and contains no
  `agent_api.control.*` section:
  - `docs/specs/universal-agent-api/capability-matrix.md:6-54`
- Current `agent_api` crate does not yet expose `run_control(...)`:
  - `crates/agent_api/src/lib.rs:138-199`

**Remediation pattern**

- **Traceability repair**: declare a single “rollout/current support” statement and make ADR/pack link
  to it; clarify how to interpret Approved specs vs Draft implementation planning.

**Doc changes applied**

- Added a canonical “Rollout / current support” section to the execution pack README, including how
  to interpret capability-matrix absence:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/README.md`
- Updated ADR-0014 to:
  - explicitly scope the Draft status as an implementation plan,
  - treat semantics as “pinned” by the canonical run protocol spec (and link it),
  - point to the execution pack README as the plan-of-record for rollout/support,
  - include the capability matrix as the current backend support signal:
  - `docs/adr/0014-agent-api-explicit-cancellation.md`
- Updated ADR drift guard:
  - `make adr-fix ADR=docs/adr/0014-agent-api-explicit-cancellation.md`

**Decisions introduced**

- None (implementation status is grounded by the capability matrix and current crate surface).

---

### CH-0003 — BH-C04/BH-C05 shorthand appears in the cancellation plan without definitions or canonical links (Fixed)

**Restated cohesion break**

SEAM-2/SEAM-4 used BH-C04/BH-C05 as if they were shared contract IDs, but no doc defined them or
linked to canonical sources, weakening seam continuity and traceability.

**Evidence**

- Backend harness posture explicitly includes drain-on-drop and DR-0012 gating integration:
  - `docs/adr/0013-agent-api-backend-harness.md:37-45`
  - `docs/adr/0013-agent-api-backend-harness.md:61-74`
- DR-0012 defines completion gating and consumer opt-out:
  - `docs/specs/universal-agent-api/run-protocol-spec.md:35-49`

**Remediation pattern**

- **Define + link**: define BH-C04/BH-C05 once in the pack and link all usages to that canonical
  definition (with pointers to authoritative sources).

**Doc changes applied**

- Added BH-C04/BH-C05 definitions (with canonical pointers) to the pack’s SEAM-2 doc:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md`
- Updated threaded SEAM-2 and SEAM-4 docs to link BH-C04/BH-C05 usages to the canonical definitions:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/seam.md`
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md`
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md`

**Decisions introduced**

- None.

---

### CH-0004 — SEAM-3 depends on SEAM-4 “pinned timeouts”, but the pack-level SEAM-4 doc does not surface them; canonical source is ambiguous (Fixed)

**Restated cohesion break**

SEAM-3 referenced SEAM-4 for “pinned timeouts” but the pack-level SEAM-4 output doc didn’t show where
those parameters lived (they were only in threaded slice docs).

**Evidence**

- Pinned timeouts exist in threaded SEAM-4 slices:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md:4-9`
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md:4-8`

**Remediation pattern**

- **Canonicalize + surface**: keep the slice docs as canonical, but surface the pinned parameters at
  the seam output level and make SEAM-3’s dependency point to the exact canonical slice docs.

**Doc changes applied**

- Surfaced the pinned timing parameters in the pack-level SEAM-4 output doc (and pointed to slice docs as canonical):
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md`
- Made SEAM-3 time-bounds dependency resolvable by naming the exact slice docs containing the pinned timeouts:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md`

**Decisions introduced**

- None.

---

### CH-0005 — Cancel-handle lifetime/orthogonality requirement is pinned in the run protocol but not traced to SEAM-2/SEAM-4 verification coverage (Fixed)

**Restated cohesion break**

The run protocol pins that `cancel()` must still function even if the caller drops `events` and/or
the run handle, but SEAM-4’s verification plan didn’t include an explicit “drop then cancel” case.

**Evidence**

- Cancel handle lifetime (orthogonal) is pinned:
  - `docs/specs/universal-agent-api/run-protocol-spec.md:87-91`

**Remediation pattern**

- **Trace + verify**: add explicit SEAM-4 coverage and link the seam requirements directly to the
  pinned run protocol rule.

**Doc changes applied**

- Added explicit reference to the run-protocol “Cancel handle lifetime (orthogonal)” rule in SEAM-2 requirements:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md`
- Added SEAM-4 coverage for “drop `events` then `cancel()`” (acceptance criteria + implementation notes):
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md`
- Surfaced the lifetime-coverage expectation at the seam level (SEAM-4 brief + pack-level SEAM-4 output doc):
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/seam.md`
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md`

**Decisions introduced**

- None.

---

### CH-0006 — Multiple ADRs cite `docs/project_management/next/universal-agent-api/*` as authoritative specs, while the canonical spec set lives under `docs/specs/universal-agent-api/` (Fixed)

**Restated cohesion break**

ADRs pointed to `docs/project_management/next/universal-agent-api/*` as “authoritative”, even though
the canonical spec set lives under `docs/specs/universal-agent-api/` (and the `next/` files are
planning-pack pointers).

**Evidence**

- Canonical spec set location is explicitly documented:
  - `docs/specs/universal-agent-api/README.md:1-13`
  - `docs/specs/universal-agent-api/contract.md:3-6`

**Remediation pattern**

- **Canonicalize references**: update ADRs to point to `docs/specs/universal-agent-api/*` for normative
  specs; keep `docs/project_management/next/` for planning/execution artifacts only.

**Doc changes applied**

- Updated ADRs to reference canonical specs under `docs/specs/universal-agent-api/`:
  - `docs/adr/0009-universal-agent-api.md`
  - `docs/adr/0010-claude-code-live-stream-json.md`
  - `docs/adr/0011-agent-api-codex-stream-exec.md`
  - `docs/adr/0012-universal-agent-api-extensions-registry-and-cli-agent-onboarding-charter.md`
- Updated ADR drift guards:
  - `make adr-fix ADR=docs/adr/0009-universal-agent-api.md`
  - `make adr-fix ADR=docs/adr/0010-claude-code-live-stream-json.md`
  - `make adr-fix ADR=docs/adr/0011-agent-api-codex-stream-exec.md`
  - `make adr-fix ADR=docs/adr/0012-universal-agent-api-extensions-registry-and-cli-agent-onboarding-charter.md`

**Decisions introduced**

- None.

## Verification

- Re-run heuristic cohesion scan after remediation (written to):
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/cohesion-audit.scan.after.json`
- Spot-check:
  - “Cancel wins” language now consistently differentiates completion value selection vs DR-0012 completion gating.
  - BH-C04/BH-C05 references resolve to a single canonical definition with pointers to ADR/spec truth.
