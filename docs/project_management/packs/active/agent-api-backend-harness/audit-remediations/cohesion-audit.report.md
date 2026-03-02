# Cohesion Audit Report

> Snapshot disclaimer (pinned):
>
> - This file is a **historical snapshot** of a cohesion audit run.
> - It **may be stale** if any file under `docs/project_management/packs/active/agent-api-backend-harness/`
>   has changed since the “Generated at” timestamp in the Meta section below.
> - Do not treat this report as normative. The normative sources of truth are the current pack docs
>   in this directory and the universal specs under `docs/specs/universal-agent-api/`.
> - Sync rule (lightweight): when changing any file in this pack, regenerate and update **all**
>   cohesion artifacts together:
>   - `cohesion-audit.report.md`
>   - `cohesion-audit.report.json`
>   - `cohesion-audit.scan.json` (or a successor scan file, if tooling changes)

## Meta
- Generated at: 2026-02-23T01:21:09Z
- Files audited: 28
- Scan used: yes (`docs/project_management/packs/active/agent-api-backend-harness/cohesion-audit.scan.json`)

## Summary
- Total issues: 5
- By severity: blocker=0, critical=1, major=2, minor=2
- High-confidence cohesion breaks: 5

## Issue index
| ID | Severity | Confidence | Type | Subject | Files |
|---|---|---|---|---|---|
| CH-0001 | major | high | terminology_drift | BH-C03 contract name/definition drifts (env-only in registry, env+timeout in slices/tasks) | docs/project_management/packs/active/agent-api-backend-harness/threading.md; docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md |
| CH-0002 | critical | high | seam_handshake_gap | SEAM-3/SEAM-4 handoff is ambiguous about completion-future ownership (single driver vs split drainer+completion tasks) | docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-1-bh-c04-drain-while-polling-completion.md; docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-2-bh-c05-canonical-handle-builder.md |
| CH-0003 | major | high | traceability_gap | Universal invalid-request checks (e.g., empty prompt) are planned without a trace to ADR/spec/current behavior, despite “no behavior change” intent | docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md; docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-3-normalization-unit-tests.md |
| CH-0004 | minor | high | canonicalization_needed | SEAM-5 still marks harness test location as TBD, despite upstream slices treating harness tests as a concrete ownership surface | docs/project_management/packs/active/agent-api-backend-harness/seam-5-backend-adoption-and-tests.md |
| CH-0005 | minor | high | definition_missing | Multiple atomic tasks leave key symbol names as TBD, reducing searchability and making cross-slice traceability weaker | docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md; docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-1-bh-c02-extension-allowlist-validator.md |

## Issues

### CH-0001 — BH-C03 contract name/definition drifts (env-only in registry, env+timeout in slices/tasks)
- Severity: major
- Confidence: high
- Type: terminology_drift
- Subject: BH-C03 contract name/definition drifts (env-only in registry, env+timeout in slices/tasks)
- Locations:
  - `docs/project_management/packs/active/agent-api-backend-harness/threading.md:20-25` (definition) — “`BH-C03 env merge precedence` … Deterministic env precedence …”
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md:10-13` (primary) — env merge + timeout derivation are both part of normalization scope
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md:6-8` (dependent) — `BH-C03` slice includes timeout derivation
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md:41-44` (reference) — claims timeout work matches BH-C03 definition in `threading.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-5-backend-adoption-and-tests/slice-1-codex-backend-migration.md:6-9` (dependent) — SEAM-5 treats env/timeout as `BH-C03`
- What breaks: The pack’s contract registry is presented as the canonical threading backbone, but `BH-C03` is defined there as env precedence only while downstream slices/tasks treat `BH-C03` as covering env + timeout semantics. This breaks registry → task traceability and makes it unclear where timeout semantics are canonically defined and tested.
- Missing links:
  - A canonical contract identifier and definition for timeout derivation semantics.
  - Consistent references in all slices/tasks to that canonical contract definition.
- Required to be cohesive:
  - Update `threading.md` so timeout derivation is either explicitly part of `BH-C03` or captured as its own `BH-C0x` contract (owner seam + definition + consumers).
  - Update SEAM-2 and SEAM-5 slice docs to match the canonical contract ID/name.
  - Ensure SEAM-2 tests explicitly list coverage for the chosen canonical timeout contract.
- Suggested evidence order: docs → codebase → git history → external → decision

### CH-0002 — SEAM-3/SEAM-4 handoff is ambiguous about completion-future ownership (single driver vs split drainer+completion tasks)
- Severity: critical
- Confidence: high
- Type: seam_handshake_gap
- Subject: SEAM-3/SEAM-4 handoff is ambiguous about completion-future ownership (single driver vs split drainer+completion tasks)
- Locations:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md:8-12` (primary) — completion polling is part of the seam’s scope
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-1-bh-c04-drain-while-polling-completion.md:6-10` (primary) — pump “polls a backend completion future concurrently”
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-2-bh-c05-canonical-handle-builder.md:33-40` (dependent) — lifecycle split: “Pump/drainer” vs “Completion sender awaits the backend completion future”
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-2-bh-c04-drain-on-drop-semantics.md:11-12` (reference) — calls for a pump “eligibility” rule but does not tie it to the SEAM-4 split explicitly
- What breaks: SEAM-3 slices describe a single pump that drains events and polls the backend completion future, while SEAM-4’s canonical builder slice describes a split design with a distinct completion task. Without a clear handshake, implementers can build incompatible designs (double-driving futures, mismatching eligibility/finality signals), risking cancellation bugs or DR-0012 gating violations.
- Missing links:
  - A canonical driver-structure decision (single task vs split tasks) referenced by both seams.
  - A clear definition of what the SEAM-3 “eligibility rule” gates (pump termination vs completion observability) vs what SEAM-4 gating enforces.
  - A consistent responsibility split note shared across SEAM-3 and SEAM-4.
- Required to be cohesive:
  - Choose and document one driver structure in a canonical location (then reference it from both seams).
  - Align SEAM-3 slice wording to match the chosen structure (avoid implying completion is polled in two places).
  - Align SEAM-4 canonical builder slice to reference the chosen SEAM-3 contract semantics (including who drops the event sender and why).
  - Add a short checklist clarifying “who owns polling completion future” and “who owns dropping sender (finality signal)”.
- Suggested evidence order: docs → codebase → git history → external → decision

### CH-0003 — Universal invalid-request checks (e.g., empty prompt) are planned without a trace to ADR/spec/current behavior, despite “no behavior change” intent
- Severity: major
- Confidence: high
- Type: traceability_gap
- Subject: Universal invalid-request checks (e.g., empty prompt) are planned without a trace to ADR/spec/current behavior, despite “no behavior change” intent
- Locations:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-5-backend-adoption-and-tests.md:25-26` (primary) — “No behavior change” intent
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md:12-13` (dependent) — proposes “prompt must be non-empty”
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-1-bh-c02-extension-allowlist-validator.md:55-60` (dependent) — call order includes universal invalid request checks
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-3-normalization-unit-tests.md:73-85` (dependent) — plans a unit test enforcing this rule
- What breaks: The plan introduces and intends to enforce at least one universal invalid-request rule, but does not provide a traceable source of truth showing it is already required by the universal spec/ADR or currently enforced by both backends. That leaves the implementer unable to tell whether the refactor preserves behavior or introduces a behavior change.
- Missing links:
  - A cited normative source (ADR/spec) or code evidence that the rule is already mandatory today.
  - If behavior differs, a bridging step describing compatibility handling or rollout.
- Required to be cohesive:
  - Either cite the exact normative location requiring the rule, or explicitly scope it as a behavior change/bugfix with rollout notes.
  - Add an explicit pre-implementation verification step comparing Codex vs Claude current behavior for empty prompt handling and record the expected harness behavior.
  - If existing backends diverge, record a decision for which behavior the harness preserves before writing harness unit tests that force convergence.
- Suggested evidence order: docs → codebase → git history → external → decision

### CH-0004 — SEAM-5 still marks harness test location as TBD, despite upstream slices treating harness tests as a concrete ownership surface
- Severity: minor
- Confidence: high
- Type: canonicalization_needed
- Subject: SEAM-5 still marks harness test location as TBD, despite upstream slices treating harness tests as a concrete ownership surface
- Locations:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-5-backend-adoption-and-tests.md:35-37` (primary) — “Harness tests (location TBD; likely …)”
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-3-streaming-pump-unit-tests.md:14-16` (dependent) — treats harness tests as a concrete location choice
- What breaks: SEAM-5 contributes to the “where does what live?” story but leaves harness test placement as TBD while upstream seams assume harness-owned tests are a concrete ownership surface. This creates avoidable friction when executing the adoption work.
- Missing links:
  - A single canonical harness test location guideline used consistently across seams.
- Required to be cohesive:
  - Pick one canonical harness test placement and reference it consistently in SEAM-5 and upstream test slices.
  - If multiple placements are valid, state the rule for unit vs integration placement and which seams own each layer.
- Suggested evidence order: docs → codebase → git history → external → decision

### CH-0005 — Multiple atomic tasks leave key symbol names as TBD, reducing searchability and making cross-slice traceability weaker
- Severity: minor
- Confidence: high
- Type: definition_missing
- Subject: Multiple atomic tasks leave key symbol names as TBD, reducing searchability and making cross-slice traceability weaker
- Locations:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md:70-72` (primary) — “names TBD”
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-1-bh-c02-extension-allowlist-validator.md:42-44` (primary) — “name TBD”
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md:47-49` (primary) — “name TBD”
- What breaks: The pack is otherwise execution-oriented (specific files, symbols, and acceptance criteria). Leaving key names as TBD reduces searchability during implementation/review and weakens cross-slice traceability when checklists are used verbatim.
- Missing links:
  - A canonical naming decision (or explicit rule that names are flexible and contracts drive review).
- Required to be cohesive:
  - Choose canonical symbol names, or replace TBD names with contract-ID-based descriptions that remain stable even if identifiers differ.
  - If naming is intentionally deferred, add one explicit naming rule (where decisions are made, and how behavior is verified without relying on names).
- Suggested evidence order: docs → codebase → git history → external → decision

## Audited files
- docs/project_management/packs/active/agent-api-backend-harness/README.md
- docs/project_management/packs/active/agent-api-backend-harness/scope_brief.md
- docs/project_management/packs/active/agent-api-backend-harness/seam-1-harness-contract.md
- docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md
- docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md
- docs/project_management/packs/active/agent-api-backend-harness/seam-4-completion-gating.md
- docs/project_management/packs/active/agent-api-backend-harness/seam-5-backend-adoption-and-tests.md
- docs/project_management/packs/active/agent-api-backend-harness/seam_map.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/seam.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-2-viability-smoke.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/seam.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-1-bh-c02-extension-allowlist-validator.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-3-normalization-unit-tests.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/seam.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-1-bh-c04-drain-while-polling-completion.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-2-bh-c04-drain-on-drop-semantics.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-3-streaming-pump-unit-tests.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/seam.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-1-bh-c05-gating-semantics.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-2-bh-c05-canonical-handle-builder.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-3-bh-c05-gating-regression-tests.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-5-backend-adoption-and-tests/seam.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-5-backend-adoption-and-tests/slice-1-codex-backend-migration.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-5-backend-adoption-and-tests/slice-2-adoption-conformance-tests.md
- docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-5-backend-adoption-and-tests/slice-3-claude-backend-migration.md
- docs/project_management/packs/active/agent-api-backend-harness/threading.md
