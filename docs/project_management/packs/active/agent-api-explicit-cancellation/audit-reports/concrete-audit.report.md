# Concrete Audit Report

Generated at: 2026-02-24T17:29:17Z

## Summary
- Files audited: 49
- Issues: 6 total (blocker 0 / critical 1 / major 3 / minor 2)

### Highest-risk gaps
1. CA-0001 — Explicit cancellation completion precedence + gating are ambiguous/inconsistent in ADR-0014 and SEAM-2 docs

### Files with highest issue density (primary locations)
- docs/adr/0014-agent-api-explicit-cancellation.md: 1
- docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md: 1
- docs/specs/universal-agent-api/capability-matrix.md: 1
- docs/project_management/packs/active/agent-api-explicit-cancellation/decision_register.md: 1
- docs/adr/0013-agent-api-backend-harness.md: 1
- docs/specs/universal-agent-api/run-protocol-spec.md: 1

## Contract inventory (explicit cancellation)
- CA-C01 — Public explicit cancellation surface
  - AgentWrapperGateway::run_control
  - AgentWrapperRunControl
  - AgentWrapperCancelHandle::cancel
  - Capability id agent_api.control.cancel.v1
  - Pinned completion error: AgentWrapperError::Backend { message: "cancelled" }
- CA-C02 — Harness cancellation propagation
  - Stop forwarding after cancel
  - Close consumer-visible events stream
  - Continue draining backend stream (drain-on-drop posture)
  - Completion selection vs completion gating
- CA-C03 — Backend termination responsibilities
  - Termination hook idempotence + non-blocking
  - No raw stdout/stderr leakage on cancellation
  - Time bounds via pinned SEAM-4 tests
- SEAM-4 — Pinned tests for cancellation + drop regression
  - Fake blocking process cancellation integration test
  - Drop receiver regression test
  - Pinned timeouts and parameters

## Issues

### CA-0001 — Explicit cancellation completion precedence + gating are ambiguous/inconsistent in ADR-0014 and SEAM-2 docs
- Severity: critical
- Category: behavior
- Location: `docs/adr/0014-agent-api-explicit-cancellation.md` L107-L123
- Excerpt: “If cancellation occurs before success completion, `completion` resolves to:”
- Problem: ADR-0014 labels its cancellation section "User Contract (Authoritative)" but conditions cancellation on "before success completion" (ambiguous: does cancellation override backend errors or only success?), and also describes cancellation enabling "early completion resolution" (ambiguous vs DR-0012 completion gating). SEAM-2 docs similarly talk about resolving completion when cancellation "wins" without explicitly restating the pinned gating rule that the cancelled completion MUST NOT resolve until the backend process has exited (and, when `events` is kept alive, until consumer-visible stream finality). This leaves implementers and tests with multiple plausible interpretations that disagree on when the cancelled completion may resolve.
- Required to be concrete:
  - Define cancellation precedence using `Result` terms everywhere cancellation semantics are described: cancellation MUST override any not-yet-resolved completion (both `Ok` and `Err`), and tie-breaking MUST be pinned for concurrent readiness (cancellation wins).
  - State explicitly (in ADR-0014 and SEAM-2 pinned docs) that cancellation completion selection is separate from completion resolution timing: the cancelled completion MUST still obey DR-0012 gating (wait for backend process exit; and wait for consumer-visible stream finality unless the consumer opts out).
  - Remove or constrain phrasing like "early completion resolution" so it cannot be read as allowing the cancelled completion to resolve before backend process exit and the DR-0012 stream-finality rule.
  - Ensure SEAM-2 pinned driver docs explicitly cross-link the run protocol’s explicit-cancellation section for both (a) consumer-visible stream closure rules after `cancel()`, and (b) completion gating requirements.
- Suggested evidence order: codebase → docs → external → decision
- Citations:
  - `docs/adr/0014-agent-api-explicit-cancellation.md` L113-L116 — “If cancellation occurs before success completion, `completion` resolves to:”
  - `docs/adr/0014-agent-api-explicit-cancellation.md` L121-L123 — “early completion resolution (error) when the run has not already completed.”
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md` L25-L27 — “the completion sender resolves completion to the pinned cancellation error if the backend does”
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md` L9-L14 — “resolve completion to the pinned cancellation error when cancel wins the race.”
  - `docs/specs/universal-agent-api/run-protocol-spec.md` L113-L118 — “Explicit cancellation is NOT an exception to the completion/event-stream finality rules above:”
- Cross-references:
  - `docs/specs/universal-agent-api/run-protocol-spec.md` L93-L118
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-2-harness-cancel-propagation.md` L15-L27
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-2-harness-cancel-propagation/slice-1-driver-semantics.md` L9-L25
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` L70-L77
- Notes: The run protocol spec is concrete; the gaps are in other "authoritative/pinned" docs that can be read as contradicting or weakening the pinned gating/precedence semantics.

### CA-0002 — SEAM-4 test contract does not pin a non-ambiguous termination signal or clearly connect seam-level requirements to the pinned slice parameters
- Severity: major
- Category: testing
- Location: `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md` L5-L12
- Excerpt: “calling `cancel()` causes the fake process to be terminated best-effort”
- Problem: The seam-level test contract states that cancellation should terminate a fake blocking process "best-effort" but does not define the observable(s) that constitute termination (process exit vs kill signal vs channel closure), nor does it pin timeouts/parameters in the seam doc. The detailed integration slice describes observing termination via the consumer-visible `events` stream reaching `None` and `completion` resolving within a timeout; however, explicit cancellation semantics also require closing the consumer-visible stream on cancel, so stream finality alone is not a unique signal of process termination. Without a pinned termination signal + pass/fail criteria (and a defined set of "supported platforms"), implementers can write tests that pass while still allowing leaked/unkilled child processes or flaky timing behavior.
- Required to be concrete:
  - Define (for SEAM-4 integration tests) the required observable(s) for "process terminated best-effort" (e.g., backend process exit observed; no child process remains; specific exit-status expectation), and specify how the test should detect this in a cross-platform way.
  - Clarify the inference chain between (a) cancelling, (b) closing the consumer-visible event stream, (c) backend process exit, and (d) completion resolution timing; document which of these are necessary and sufficient for the test to treat termination as successful.
  - Ensure `seam-4-tests.md` either (a) includes the pinned parameters/timeouts (`FIRST_EVENT_TIMEOUT`, `CANCEL_TERMINATION_TIMEOUT`, `DROP_COMPLETION_TIMEOUT`, `MANY_EVENTS_N`) and pass/fail criteria, or (b) explicitly declares the slice docs as the authoritative source for those pinned parameters.
  - Define "supported platforms" (OS/arch set) or link to a single authoritative repo policy so statements like "same timeouts on all supported platforms" are concrete and enforceable.
- Suggested evidence order: codebase → docs → external → decision
- Citations:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-4-tests.md` L7-L10 — “calling `cancel()` causes the fake process to be terminated best-effort”
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md` L6-L9 — “`CANCEL_TERMINATION_TIMEOUT`: `3s` (after calling `cancel()`, both:”
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md` L24-L26 — “Calling `cancel()` causes the fake backend process to be terminated best-effort, observed via:”
  - `docs/specs/universal-agent-api/run-protocol-spec.md` L93-L98 — “Once cancellation is requested, the backend MUST:”
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md` L41-L43 — “Built-in backends MUST satisfy those tests on supported platforms.”
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md` L4-L9 — “These timeouts are the same on all supported platforms (no platform-specific adjustment in v1).”
- Cross-references:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-1-explicit-cancel-integration.md` L4-L33
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/threaded-seams/seam-4-tests/slice-2-drop-regression.md` L4-L25
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-3-backend-termination.md` L41-L43
  - `docs/specs/universal-agent-api/run-protocol-spec.md` L93-L118
- Notes: This issue is about making the test contract itself concrete (what to assert), not about choosing an implementation strategy for termination.

### CA-0003 — Capability matrix does not define absence semantics for standard capability ids (explicit cancellation capability missing)
- Severity: major
- Category: consistency
- Location: `docs/specs/universal-agent-api/capability-matrix.md` L9-L40
- Excerpt: “## `agent_api.exec`”
- Problem: `capabilities-schema-spec.md` defines `agent_api.control.cancel.v1` as a standard universal capability id with minimum semantics. The generated/approved capability matrix currently lists several `agent_api.*` capability groups but does not include any `agent_api.control.*` section or a row for `agent_api.control.cancel.v1`. Without a pinned rule for what matrix omission means, consumers and implementers cannot tell whether the capability is unsupported, not yet shipped, or simply omitted by the generator unless advertised by a backend.
- Required to be concrete:
  - Define what it means when a standard capability id is absent from the generated matrix (e.g., matrix lists only capability ids currently advertised by at least one backend; absence means no backend advertises it).
  - Decide and document whether the matrix is required to include all standard `agent_api.*` capability ids (including those currently unsupported) vs only advertised capability ids.
  - Ensure the explicit cancellation documentation clearly directs runtime availability checks to `AgentWrapperCapabilities.ids` (capability presence on the backend), and clarifies the matrix’s scope so consumers do not treat it as an exhaustive registry.
- Suggested evidence order: codebase → docs → external → decision
- Citations:
  - `docs/specs/universal-agent-api/capability-matrix.md` L9-L39 — “## `agent_api.core`”
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md` L80-L84 — “- `agent_api.control.cancel.v1`:”
- Cross-references:
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md` L76-L85
  - `docs/specs/universal-agent-api/run-protocol-spec.md` L77-L85
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` L57-L62
- Notes: This is about making the documentation of the generated artifact’s semantics concrete; it does not require changing what any backend advertises.

### CA-0006 — `run-protocol-spec.md` uses non-testable qualifiers for capability validation timing and error-event emission
- Severity: major
- Category: behavior
- Location: `docs/specs/universal-agent-api/run-protocol-spec.md` L120-L125
- Excerpt: “validate capabilities before spawning work where possible”
- Problem: The run protocol spec is normative, but the capability validation timing section uses qualifiers ("where possible", "if feasible") without defining the conditions under which they apply or the required fallback behavior. This makes the requirement non-concrete and not directly testable: two compliant implementations could validate at different times and disagree on whether an `Error` event is required.
- Required to be concrete:
  - Define concrete rules for when capability validation MUST occur relative to spawning backend work (including explicit exceptions, if any).
  - Define when an `AgentWrapperEventKind::Error` event MUST be emitted on unsupported operations (e.g., only if the consumer-visible stream is still open), and specify the required behavior when emission is not possible.
  - If the intent is guidance rather than a requirement, downgrade the language to non-normative and/or provide an explicit testability note.
- Suggested evidence order: codebase → docs → external → decision
- Citations:
  - `docs/specs/universal-agent-api/run-protocol-spec.md` L122-L125 — “validate capabilities before spawning work where possible”
  - `docs/specs/universal-agent-api/run-protocol-spec.md` L124-L125 — “emit an `Error` event if feasible”
- Cross-references:
  - `docs/specs/universal-agent-api/contract.md` L169-L191
  - `docs/specs/universal-agent-api/extensions-spec.md` L39-L56
- Notes: This affects cancellation indirectly because explicit cancellation adds another capability-gated operation (`run_control(...)`).

### CA-0004 — DR-CA-0002 uses a `<pinned string>` placeholder instead of the pinned cancellation message
- Severity: minor
- Category: language
- Location: `docs/project_management/packs/active/agent-api-explicit-cancellation/decision_register.md` L19-L28
- Excerpt: “Option A: represent cancellation as `AgentWrapperError::Backend { message: <pinned string> }`.”
- Problem: The decision register is meant to record the selected, concrete A/B decision, but uses a placeholder (`<pinned string>`) for the cancellation error message. Elsewhere in the same docset the pinned message is specified exactly as `"cancelled"`.
- Required to be concrete:
  - Replace `<pinned string>` with the exact pinned message (`"cancelled"`) and ensure spelling/casing is consistent across the pack and canonical specs.
  - Add an explicit pointer to the canonical pinned string definition (SEAM-1 and/or `run-protocol-spec.md`) so the DR cannot drift.
- Suggested evidence order: codebase → docs → external → decision
- Citations:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/decision_register.md` L21-L23 — “Option A: represent cancellation as `AgentWrapperError::Backend { message: <pinned string> }`.”
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` L72-L73 — “- `Err(AgentWrapperError::Backend { message })` where `message == "cancelled"`.”
- Cross-references:
  - `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` L70-L74
  - `docs/specs/universal-agent-api/run-protocol-spec.md` L104-L108
- Notes: This is a documentation concreteness issue; it does not require changing the chosen (stringly-typed) error representation.

### CA-0005 — ADR-0013 leaves the backend harness module name as TBD
- Severity: minor
- Category: language
- Location: `docs/adr/0013-agent-api-backend-harness.md` L96-L122
- Excerpt: “Add an internal module (name TBD; e.g. `agent_api::backend_harness`) which implements the common”
- Problem: ADR-0013 includes a placeholder for a central architectural boundary (“name TBD”), while also listing concrete file/module boundaries later in the same section. Leaving the module name undecided makes the ADR non-concrete and increases the risk of downstream docs/code referencing different module paths during implementation.
- Required to be concrete:
  - Pin the module name and primary file path (or explicitly defer with a tracked decision) so there is exactly one canonical internal harness module path.
  - Ensure the "File/module boundaries" section uses the same pinned module name/path as the architecture section.
- Suggested evidence order: codebase → docs → external → decision
- Citations:
  - `docs/adr/0013-agent-api-backend-harness.md` L98-L99 — “Add an internal module (name TBD; e.g. `agent_api::backend_harness`) which implements the common”
  - `docs/adr/0013-agent-api-backend-harness.md` L120-L122 — “- `crates/agent_api/src/backend_harness.rs` (new; internal)”
- Notes: The missing decision is about naming/structure, not about altering any public contract.

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
