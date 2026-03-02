# Cohesion Audit Report

## Meta
- Generated at: 2026-02-24T16:41:13Z
- Files audited: 49
- Scan used: yes (`docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/cohesion-audit.scan.json`)

## Summary
- Total issues: 6
- By severity: blocker=0, critical=1, major=5, minor=0
- High-confidence cohesion breaks: 6

## Issue index
| ID | Severity | Confidence | Type | Subject | Files |
|---|---|---|---|---|---|
| CH-0001 | major | high | scope_disconnect | Explicit cancellation appears as both Draft (ADR/pack) and Approved (canonical specs); current support/rollout state is unclear | docs/adr/0014-agent-api-explicit-cancellation.md; docs/specs/universal-agent-api/contract.md; docs/specs/universal-agent-api/capability-matrix.md |
| CH-0002 | critical | high | missing_bridge_step | Cancellation completion outcome selection is not clearly bridged to DR-0012 completion gating (must wait for backend process exit) | docs/specs/universal-agent-api/run-protocol-spec.md; docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md; docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md |
| CH-0003 | major | high | definition_missing | BH-C04/BH-C05 shorthand appears in the cancellation plan without definitions or canonical links | docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md; docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md; docs/specs/universal-agent-api/run-protocol-spec.md |
| CH-0004 | major | high | canonicalization_needed | SEAM-3 depends on SEAM-4 “pinned timeouts”, but the pack-level SEAM-4 doc does not surface them; canonical source is ambiguous | docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md; docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md; docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md |
| CH-0005 | major | high | verification_gap | Cancel-handle lifetime/orthogonality requirement is pinned in the run protocol but not traced to SEAM-2/SEAM-4 verification coverage | docs/specs/universal-agent-api/run-protocol-spec.md; docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md; docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md |
| CH-0006 | major | high | canonicalization_needed | Multiple ADRs cite `docs/project_management/next/universal-agent-api/*` as authoritative specs, while the canonical spec set lives under `docs/specs/universal-agent-api/` | docs/specs/universal-agent-api/README.md; docs/adr/0009-universal-agent-api.md; docs/adr/0011-agent-api-codex-stream-exec.md |

## Issues

### CH-0001 — Explicit cancellation lifecycle state unclear (Draft ADR/pack vs Approved specs)
- Severity: major
- Confidence: high
- Type: scope_disconnect
- Subject: Explicit cancellation appears as both Draft (ADR/pack) and Approved (canonical specs); current support/rollout state is unclear
- Locations:
  - `docs/adr/0014-agent-api-explicit-cancellation.md:6-9` (primary) — “Status: Draft”
  - `docs/adr/0014-agent-api-explicit-cancellation.md:140-148` (dependent) — “Semantics (to be pinned before implementation)”
  - `docs/specs/universal-agent-api/contract.md:3-6` (dependent) — “Status: Approved” / “Canonical location: `docs/specs/universal-agent-api/`”
  - `docs/specs/universal-agent-api/run-protocol-spec.md:72-86` (dependent) — “Explicit cancellation semantics (v1, normative)”
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md:80-84` (reference) — `agent_api.control.cancel.v1`
  - `docs/specs/universal-agent-api/capability-matrix.md:9-40` (reference) — generated capability sections (no `agent_api.control.*` section shown)
- What breaks: The docset sends incompatible lifecycle signals: explicit cancellation is “Draft” and “to be pinned before implementation” in ADR-0014, while the canonical spec set is “Approved” and already defines explicit cancellation semantics and surface types. Readers cannot tell whether cancellation is already part of the plan-of-record (shipped/stable) or a planned change, and whether the capability is expected to appear in the capability matrix today.
- Missing links:
  - A single canonical statement describing the current rollout/support state for `agent_api.control.cancel.v1` (planned vs shipped; per-backend support).
  - Clear guidance for how to interpret “Approved” spec status vs “Draft” ADR status for the same feature.
  - A clear explanation for the capability matrix not including any `agent_api.control.*` section despite `agent_api.control.cancel.v1` being a standard capability id.
- Required to be cohesive:
  - Declare one canonical location to state rollout/support status for explicit cancellation and reference it from ADR-0014 and the execution pack.
  - Align ADR-0014’s “to be pinned” section with the run-protocol spec’s already-pinned cancellation semantics (either by pointing to the spec as canonical or scoping ADR-0014 as proposing changes not yet reflected in the approved spec).
  - Clarify (or pin) how capability matrix omission should be interpreted for standard capability ids like `agent_api.control.cancel.v1`.
- Suggested evidence order: docs → codebase → git history → external → decision

### CH-0002 — Cancellation outcome selection is not bridged to DR-0012 completion gating
- Severity: critical
- Confidence: high
- Type: missing_bridge_step
- Subject: Cancellation completion outcome selection is not clearly bridged to DR-0012 completion gating (must wait for backend process exit)
- Locations:
  - `docs/specs/universal-agent-api/run-protocol-spec.md:35-49` (definition) — DR-0012 completion gating (“completion MUST NOT resolve until … process has exited … stream finality”)
  - `docs/specs/universal-agent-api/run-protocol-spec.md:113-118` (definition) — explicit cancellation gating (“NOT an exception … MUST NOT resolve until … process has exited”)
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md:70-76` (primary) — cancellation outcome/precedence pinned
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md:22-27` (primary) — completion sender resolves to pinned error “if the backend does not complete first”
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md:9-14` (dependent) — “resolve completion … when cancel wins the race”
- What breaks: The run protocol pins completion gating, but the execution docs emphasize choosing the pinned `"cancelled"` completion outcome when cancellation wins without consistently restating that cancellation does not relax DR-0012 timing constraints. This allows multiple plausible (incompatible) implementations/tests regarding whether cancellation can resolve completion before backend process exit.
- Missing links:
  - A seam-level bridge that explicitly states cancellation changes the completion *value* but not the completion *gating* rules.
  - Verification language that asserts backend process exit gating for cancellation completion (not only stream closure/no late events).
  - Direct cross-links from SEAM-1/SEAM-2 docs to the run-protocol explicit cancellation section that pins both precedence and gating.
- Required to be cohesive:
  - Restate DR-0012 gating for the cancellation path in SEAM-1/SEAM-2 docs (wait for process exit; wait for stream finality unless consumer opts out).
  - Ensure SEAM-4 tests assert the gating constraints under cancellation (within pinned timeouts).
  - Qualify “cancel wins” language so it cannot be read as allowing completion to resolve before process exit.
- Suggested evidence order: docs → codebase → git history → external → decision

### CH-0003 — BH-C04/BH-C05 shorthand appears without definitions/canonical links
- Severity: major
- Confidence: high
- Type: definition_missing
- Subject: BH-C04/BH-C05 shorthand appears in the cancellation plan without definitions or canonical links
- Locations:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md:7-11` (primary) — “receiver (BH-C04 posture)” / “Explicit cancellation must be orthogonal”
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md:10-14` (dependent) — “keep draining … (BH-C04)”
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md:11-13` (dependent) — “DR-0012/BH-C05 opt-out behavior”
  - `docs/specs/universal-agent-api/run-protocol-spec.md:35-49` (reference) — DR-0012 completion gating definitions/rules
- What breaks: SEAM-2/SEAM-4 rely on BH-C04/BH-C05 as if they are shared contract IDs, but the explicit cancellation docset does not define them or link to canonical definitions. This breaks navigation and traceability for implementers trying to understand the drain-on-drop posture and completion gating assumptions the cancellation plan depends on.
- Missing links:
  - Definitions for BH-C04 and BH-C05 within the explicit cancellation pack (or resolvable links to canonical definitions elsewhere).
  - A mapping from BH-C05 references to the DR-0012 section in `run-protocol-spec.md`.
- Required to be cohesive:
  - Define BH-C04/BH-C05 (or replace them with explicit references) in a canonical location within this pack.
  - Update all BH-C04/BH-C05 references to include resolvable links to the canonical definitions.
  - Ensure BH-C05 references explicitly link to DR-0012 so the plan is executable without implicit context.
- Suggested evidence order: docs → codebase → git history → external → decision

### CH-0004 — SEAM-3 depends on SEAM-4 pinned timeouts, but pack-level SEAM-4 doc omits them
- Severity: major
- Confidence: high
- Type: canonicalization_needed
- Subject: SEAM-3 depends on SEAM-4 “pinned timeouts”, but the pack-level SEAM-4 doc does not surface them; canonical source is ambiguous
- Locations:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md:41-43` (primary) — “defined by SEAM-4 tests (pinned timeouts…)”
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md:5-13` (dependent) — seam-level required tests list (no pinned timeouts here)
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/seam.md:28-30` (definition) — pinned timeouts live in slice docs
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md:4-9` (definition) — `FIRST_EVENT_TIMEOUT=1s`, `CANCEL_TERMINATION_TIMEOUT=3s`
- What breaks: SEAM-3’s termination requirements defer to SEAM-4 for time-bounded pass/fail criteria, but the seam-level SEAM-4 doc listed in the seam map doesn’t contain the pinned parameters. Readers must discover them in threaded slice docs without a canonical pointer, breaking seam-to-seam continuity.
- Missing links:
  - A precise reference from SEAM-3 to the exact SEAM-4 slice doc(s) that define pinned timeouts and pass/fail criteria.
  - A seam-level pointer in `seam-4-tests.md` indicating the canonical location of pinned parameters.
- Required to be cohesive:
  - Make the SEAM-3 → SEAM-4 dependency resolvable by linking to the canonical pinned-parameter locations.
  - Ensure `seam-4-tests.md` either contains the pinned parameters or links to the threaded slice docs as canonical.
- Suggested evidence order: docs → codebase → git history → external → decision

### CH-0005 — Cancel-handle lifetime requirement is not traced to explicit verification coverage
- Severity: major
- Confidence: high
- Type: verification_gap
- Subject: Cancel-handle lifetime/orthogonality requirement is pinned in the run protocol but not traced to SEAM-2/SEAM-4 verification coverage
- Locations:
  - `docs/specs/universal-agent-api/run-protocol-spec.md:87-91` (definition) — cancel must work even after dropping `events` and/or dropping the run handle
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md:9-11` (primary) — “Explicit cancellation must be orthogonal”
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md:15-20` (dependent) — integration test plan calls `cancel()` but does not pin drop-then-cancel coverage
- What breaks: The run protocol defines a concrete cancel-handle lifetime guarantee, but the execution pack’s verification coverage does not explicitly prove it. This breaks traceability from normative requirement → test plan and leaves a gap that could allow regressions (cancel handle accidentally tied to event receiver lifetime).
- Missing links:
  - An explicit SEAM-4 test/acceptance criterion for drop-then-cancel behavior.
  - A direct trace from the run-protocol lifetime rule to the SEAM-4 test(s) that prove it.
- Required to be cohesive:
  - Add explicit SEAM-4 verification for cancel-handle lifetime (drop events and/or handle, then cancel and assert pinned outcomes).
  - Link SEAM-2/SEAM-4 docs directly to the run-protocol lifetime section.
- Suggested evidence order: docs → codebase → git history → external → decision

### CH-0006 — Universal Agent API spec canonicalization is split between `next/` and `docs/specs/`
- Severity: major
- Confidence: high
- Type: canonicalization_needed
- Subject: Multiple ADRs cite `docs/project_management/next/universal-agent-api/*` as authoritative specs, while the canonical spec set lives under `docs/specs/universal-agent-api/`
- Locations:
  - `docs/specs/universal-agent-api/README.md:1-13` (definition) — canonical spec directory under `docs/specs/universal-agent-api/`
  - `docs/specs/universal-agent-api/contract.md:3-6` (definition) — “Canonical location: `docs/specs/universal-agent-api/`”
  - `docs/adr/0009-universal-agent-api.md:25-30` (primary) — “Contract/spec docs (authoritative)” under `docs/project_management/next/universal-agent-api/`
  - `docs/adr/0011-agent-api-codex-stream-exec.md:37-43` (dependent) — baseline universal contract references `docs/project_management/next/universal-agent-api/*`
- What breaks: Readers encounter two competing “authoritative” spec trees for the Universal Agent API. Without an explicit mapping, this risks drift and breaks narrative cohesion when one tree updates independently.
- Missing links:
  - An explicit statement of the relationship between the `next/` spec tree and the `docs/specs/` canonical spec tree.
  - Consistent ADR references to the canonical location for normative contracts.
- Required to be cohesive:
  - Choose and document exactly one canonical location for normative Universal Agent API specs and reference it consistently from ADRs.
  - If `docs/project_management/next/universal-agent-api/*` remains, scope it as planning/execution artifacts and state how it is synchronized (or that it is not).
  - Add cross-links so readers can navigate ADR → canonical spec → planning artifacts without ambiguity.
- Suggested evidence order: docs → codebase → git history → external → decision

## Audited files
- docs/project_management/packs/active/agent-api-explicit-cancellation/README.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/decision_register.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/scope_brief.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/seam_map.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/seam.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/slice-1-canonical-contracts.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-1-cancellation-contract/slice-2-agent-api-surface.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/seam.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-2-harness-control-entrypoint.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-3-backend-termination/seam.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-3-backend-termination/slice-1-backend-adoption.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-3-backend-termination/slice-2-termination-hooks.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/seam.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md
- docs/project_management/packs/active/agent-api-explicit-cancellation/threading.md
- docs/adr/0001-codex-cli-parity-maintenance.md
- docs/adr/0002-codex-cli-parity-coverage-mapping.md
- docs/adr/0003-wrapper-coverage-auto-generation.md
- docs/adr/0004-wrapper-coverage-iu-subtree-inheritance.md
- docs/adr/0005-codex-jsonl-log-parser-api.md
- docs/adr/0006-agent-wrappers-workspace.md
- docs/adr/0007-wrapper-events-ingestion-contract.md
- docs/adr/0008-claude-stream-json-parser-api.md
- docs/adr/0009-universal-agent-api.md
- docs/adr/0010-claude-code-live-stream-json.md
- docs/adr/0011-agent-api-codex-stream-exec.md
- docs/adr/0012-universal-agent-api-extensions-registry-and-cli-agent-onboarding-charter.md
- docs/adr/0013-agent-api-backend-harness.md
- docs/adr/0014-agent-api-explicit-cancellation.md
- docs/specs/claude-stream-json-parser-contract.md
- docs/specs/claude-stream-json-parser-scenarios-v1.md
- docs/specs/codex-thread-event-jsonl-parser-contract.md
- docs/specs/codex-thread-event-jsonl-parser-scenarios-v1.md
- docs/specs/codex-wrapper-coverage-generator-contract.md
- docs/specs/codex-wrapper-coverage-scenarios-v1.md
- docs/specs/universal-agent-api/README.md
- docs/specs/universal-agent-api/capabilities-schema-spec.md
- docs/specs/universal-agent-api/capability-matrix.md
- docs/specs/universal-agent-api/contract.md
- docs/specs/universal-agent-api/event-envelope-schema-spec.md
- docs/specs/universal-agent-api/extensions-spec.md
- docs/specs/universal-agent-api/run-protocol-spec.md
- docs/specs/wrapper-events-ingestion-contract.md
