# Contradictions Audit Report

## Meta
- Generated at: 2026-02-24T17:16:17Z
- Files audited: 49
- Scan used: yes (`docs/project_management/packs/active/agent-api-explicit-cancellation/audit-reports/contradictions-audit.scan.json`)

## Summary
- Total issues: 1
- By severity: blocker=0, critical=0, major=1, minor=0
- High-confidence contradictions: 0

## Issue index
| ID | Severity | Confidence | Type | Subject | Files |
|---|---|---|---|---|---|
| CX-0001 | major | medium | scope_mismatch | ADR-0014: drop semantics vs cancellation meaning | docs/adr/0014-agent-api-explicit-cancellation.md |

## Issues

### CX-0001 — Drop semantics wording conflicts on “cancel”
- Severity: major
- Confidence: medium
- Type: scope_mismatch
- Subject: ADR-0014: drop semantics vs cancellation meaning
- Scope: environment=all, version=unknown, feature_flag=none, timeline=planned
- Statement A: `docs/adr/0014-agent-api-explicit-cancellation.md:66-69` — “dropping must not imply cancel, but consumers still need a supported way to cancel intentionally.”
- Statement B: `docs/adr/0014-agent-api-explicit-cancellation.md:142-144` — “Drop semantics remain “best-effort cancellation”… not required to be reliable.”
- Why this conflicts: Within the same ADR, one statement reads as “dropping must not imply cancel”, while a later section states that drop semantics remain “best-effort cancellation” per the run protocol. These can both be true only if the earlier “cancel” refers specifically to explicit/intentional cancellation (not drop-based best-effort cancellation), but that scope qualifier is not stated, leading to incompatible reader interpretations.
- What must be true:
  - Whether dropping `events` / the run handle is specified as a cancellation signal (even best-effort) or only as a consumer opt-out, and how the term “cancel” is scoped in ADR-0014.
- Suggested evidence order:
  - codebase
  - tests
  - runtime-config
  - git-history
  - other-docs
  - external
  - decision

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
