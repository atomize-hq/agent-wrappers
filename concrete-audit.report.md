# Concrete Audit Report

Generated at: 2026-02-22T20:59:03-05:00

## Summary
- Files audited: 33
- Issues: 18 total (blocker 3 / critical 6 / major 8 / minor 1)

### Highest-risk gaps
1. CA-0001 — BH-C01 harness adapter interface is described but not specified as an exact Rust API
2. CA-0002 — NormalizedRequest is required, but its schema and adapter-policy flow are not defined
3. CA-0003 — Backend stream/completion error propagation model is implied but not specified
4. CA-0004 — Error mapping/redaction contract is described but missing concrete rules and payload limits
5. CA-0005 — BH-C03 timeout precedence is stated inconsistently (cannot implement deterministically)
6. CA-0006 — Timeout derivation is specified, but timeout representation and enforcement behavior are not
7. CA-0007 — Pump backpressure behavior is required but left as “define what happens under backpressure”
8. CA-0008 — Mapping hook semantics (0..N events, ordering, bounds, mapping failures) are not pinned
9. CA-0009 — Harness driver task ownership/cancellation semantics are vague (risk of premature drop or leaks)

### Files with highest issue density (primary locations)
1. `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md` — 4
2. `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md` — 2
3. `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-1-bh-c02-extension-allowlist-validator.md` — 1
4. `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md` — 1
5. `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-2-bh-c04-drain-on-drop-semantics.md` — 1

## Issues

### CA-0001 — BH-C01 harness adapter interface is described but not specified as an exact Rust API
- Severity: blocker
- Category: contract
- Location: `docs/project_management/packs/active/agent-api-backend-harness/seam-1-harness-contract.md` L8-L24
- Excerpt: “Define the internal harness entrypoint(s) and the adapter-facing interface (trait or function bundle)...”
- Problem: The docs explain what BH-C01 must cover, but never pin an exact trait/struct name, method signatures, associated types, bounds, and the harness entrypoint signature. Downstream seams depend on this shape; without it, implementers must invent an API, risking drift and rework.
- Required to be concrete:
  - Specify the canonical Rust interface name(s) for BH-C01 (trait/struct) and the harness entrypoint function signature
  - Define each required method signature, including inputs/outputs, associated types, bounds (Send/Sync/'static), and ownership/lifetime rules for stream and completion
  - Define the spawn return shape precisely (types, error types, and how the typed event stream and completion future are produced)
  - Define the mapping hook signature (inputs, output shape, and how multiple events are represented)
  - Define how the harness obtains `agent_kind`/identity from the adapter (type and formatting)
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md` L31-L43
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-2-bh-c05-canonical-handle-builder.md` L6-L17

### CA-0002 — NormalizedRequest is required, but its schema and adapter-policy flow are not defined
- Severity: blocker
- Category: schema
- Location: `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md` L18-L28
- Excerpt: “Outputs: A “normalized request” struct (internal) used by the harness to spawn.”
- Problem: Multiple seams require a normalized request shape, but the docs never list its fields, types, required/optional values, defaults, or how backend-specific extracted policy is carried from validation into spawn. This blocks writing deterministic tests and makes it easy for backends to continue re-deriving semantics.
- Required to be concrete:
  - List the exact fields of the internal NormalizedRequest (and their Rust types), including env, timeout, and any other spawn inputs
  - Define which raw inputs are preserved vs discarded, and where redacted logging may read from
  - Define how backend-specific extracted policy is represented (type parameter/associated type/boxed trait object) and how it reaches spawn
  - Define defaults for all optional/absent values (including explicit representation of “absent timeout”)
  - Define the normalization function signature that returns NormalizedRequest (and error type)
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md` L77-L89
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-1-bh-c02-extension-allowlist-validator.md` L55-L66

### CA-0003 — Backend stream/completion error propagation model is implied but not specified
- Severity: blocker
- Category: behavior
- Location: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md` L39-L42
- Excerpt: “Stream<Item = Result<TypedEvent, BackendErr>>, Future<Output = Result<TypedCompletion, BackendErr>>”
- Problem: The typed stream and completion future are modeled as Result-bearing, but the docs do not specify what the harness must do on `BackendErr` from the stream or completion: whether to emit an error event, terminate forwarding, continue draining, resolve completion with an error, or how to reconcile stream-error vs completion-error races.
- Required to be concrete:
  - Specify the harness behavior when the typed event stream yields an error (terminate vs continue, whether an error is emitted as an event, and finality signaling)
  - Specify the harness behavior when the completion future yields an error (how it is mapped and when it becomes observable relative to finality)
  - Specify which error source wins if both stream and completion fail (and how that is tested deterministically)
  - Specify whether mapping/bounds are applied to error events (if any) and whether the backend process must still be drained after an error
  - Specify how these behaviors interact with DR-0012 gating and the consumer-drop escape hatch
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md` L19-L31
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-2-bh-c05-canonical-handle-builder.md` L33-L39

### CA-0004 — Error mapping/redaction contract is described but missing concrete rules and payload limits
- Severity: critical
- Category: security
- Location: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md` L76-L93
- Excerpt: “Define canonical error-mapping points (redaction boundary)... prevent leaking raw backend lines...”
- Problem: The docs mandate redaction and stable errors, but do not define concrete mapping rules: which `AgentWrapperError` variants are used per phase, what fields are included, what text is allowed, and how to bound message sizes/content. This is a correctness and security gap (leakage risk).
- Required to be concrete:
  - Specify the exact mapping API (function/trait name and signature), including the phase representation
  - Define per-phase mapping rules (spawn/stream/completion): which error variant(s) are produced and which fields are populated
  - Define redaction requirements (what must never appear) and bounding rules (max sizes, truncation strategy, stable formatting expectations)
  - Define whether mapping may emit `AgentWrapperEvent` error events vs returning an error, and when each is used
  - Add at least one pinned test case per boundary that proves raw backend output is not surfaced
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-5-backend-adoption-and-tests.md` L24-L27

### CA-0005 — BH-C03 timeout precedence is stated inconsistently (cannot implement deterministically)
- Severity: critical
- Category: consistency
- Location: `docs/project_management/packs/active/agent-api-backend-harness/threading.md` L20-L26
- Excerpt: “Timeout precedence: request timeout override < backend default timeout”
- Problem: The threading contract registry appears to invert precedence ("request override < backend default") while SEAM-2 repeatedly states “request timeout overrides backend default.” This is a concrete behavior decision; conflicting statements block implementers from writing correct code and tests.
- Required to be concrete:
  - Restate timeout precedence unambiguously using an explicit effective-timeout expression (e.g., request overrides default, or the opposite)
  - Define what “override” means when the request explicitly specifies “no timeout” (and how that differs from absent)
  - Define the behavior for all four combinations: request present/absent × default present/absent
  - Ensure all mentions of timeout precedence across the pack match the chosen rule
  - Pin the chosen semantics with the SEAM-2 unit tests described in S3
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md` L10-L13
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md` L65-L67

### CA-0006 — Timeout derivation is specified, but timeout representation and enforcement behavior are not
- Severity: critical
- Category: behavior
- Location: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md` L53-L67
- Excerpt: “normalize to a single internal representation... wiring of enforcement happens where the harness awaits spawn/stream/completion”
- Problem: The docs defer key decisions: the internal timeout type/units, which phases are timed (spawn/stream/completion), and what happens on timeout (cancellation semantics and error variants). This is required to preserve “no behavior change” during SEAM-5 migration.
- Required to be concrete:
  - Specify the internal timeout representation and units (exact Rust type)
  - Specify which operations are subject to timeout (spawn, stream drain, completion await, or overall run)
  - Specify the exact timeout failure behavior (error variant, message fields, and whether backend processes/streams are cancelled or drained)
  - Specify how “absent timeout” is represented and how it interacts with explicit “no timeout” requests (if supported)
  - Add at least one pinned test that proves the enforcement behavior and error mapping are stable
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md` L25-L25

### CA-0007 — Pump backpressure behavior is required but left as “define what happens under backpressure”
- Severity: critical
- Category: behavior
- Location: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-2-bh-c04-drain-on-drop-semantics.md` L77-L89
- Excerpt: “Define what happens under backpressure (e.g., await send while receiver is alive; once dropped, stop forwarding and drain).”
- Problem: Backpressure is explicitly called out as a deadlock risk, but the pack does not choose an algorithm or a guaranteed behavior. This is especially problematic because “keep draining” and “await send” can be in tension while the receiver is alive.
- Required to be concrete:
  - Specify the exact backpressure algorithm while the receiver is alive (block, drop, batch, or decouple) and justify it against the “keep draining” invariant
  - Specify whether event ordering is preserved under backpressure and whether any events may be dropped while the receiver is alive
  - Specify how the pump detects receiver drop and transitions behavior under backpressure
  - Specify how bounds enforcement interacts with the chosen algorithm (per-event vs pre-batch)
  - Add a pinned regression test that would fail if the chosen backpressure behavior changes
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md` L8-L15

### CA-0008 — Mapping hook semantics (0..N events, ordering, bounds, mapping failures) are not pinned
- Severity: critical
- Category: contract
- Location: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-1-bh-c04-drain-while-polling-completion.md` L30-L39
- Excerpt: “Ensure the mapping hook can emit 0..N AgentWrapperEvents per backend event...”
- Problem: The pump contract allows mapping to emit multiple events per backend event, but does not specify ordering requirements, whether mapping may be fallible, how to handle mapping errors, or whether bounds/redaction apply before or after mapping. This blocks writing deterministic pump and migration tests.
- Required to be concrete:
  - Specify whether mapping is infallible or may return an error, and define the harness behavior on mapping failure
  - Specify ordering guarantees for emitted events when mapping returns 0..N events (within a source event and across source events)
  - Specify whether bounds/redaction are applied per emitted event and whether mapping receives raw or already-bounded data
  - Specify whether mapping may emit “error events” and how those interact with completion errors and DR-0012 gating
  - Add at least one pinned test asserting the chosen ordering and mapping-error behavior
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md` L20-L23
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md` L31-L31

### CA-0009 — Harness driver task ownership/cancellation semantics are vague (risk of premature drop or leaks)
- Severity: critical
- Category: operational
- Location: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-2-bh-c05-canonical-handle-builder.md` L33-L40
- Excerpt: “Ensure the driver task is not dropped early (store join handle if needed, or structure it so the pump + completion outlive the caller).”
- Problem: The pack requires correctness-sensitive background tasks (pump, completion sender), but does not define who owns them, what triggers cancellation, and what happens when `AgentWrapperRunHandle` is dropped. This can cause premature cancellation (breaking drain/gating semantics) or leaked tasks.
- Required to be concrete:
  - Specify task ownership: where JoinHandles live and what object drops them (if ever)
  - Specify the cancellation policy when the run handle is dropped vs when the consumer drops only the events stream
  - Specify shutdown behavior for the pump and completion sender (normal completion, errors, timeouts)
  - Specify any resource cleanup requirements (backend process termination vs continued drain) consistent with ADR-0013
  - Pin the policy with at least one test that would fail if tasks are dropped prematurely
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-4-completion-gating.md` L19-L21

### CA-0010 — Fail-closed extension allowlist validator requires deterministic “first unknown key” selection but does not define ordering
- Severity: major
- Category: behavior
- Location: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-1-bh-c02-extension-allowlist-validator.md` L28-L35
- Excerpt: “returns UnsupportedCapability deterministically on the first unknown key... avoid “random first key” from hash iteration”
- Problem: The requirement for stable selection is explicit, but the ordering rule is not. If `extensions` is a map type with non-deterministic iteration, implementers must choose whether to sort keys, preserve insertion order, or use a stable iteration structure; tests cannot be written without this decision.
- Required to be concrete:
  - Specify the ordering rule used to pick the “first unknown key” (e.g., lexicographic sort by key, or preserve request order)
  - Specify whether the validator stops at the first unknown key or reports all unknown keys (and the exact error payload shape)
  - Specify whether allowlist comparison is exact-match and whether keys are normalized (case sensitivity, trimming, Unicode normalization)
  - Add a unit test that would fail if key selection ordering changes
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-3-normalization-unit-tests.md` L30-L41

### CA-0011 — Extension key matching rules are not defined (case sensitivity, duplicates, namespaces)
- Severity: major
- Category: schema
- Location: `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md` L8-L9
- Excerpt: “Centralize fail-closed extension key validation against a backend-provided allowlist.”
- Problem: The allowlist validator is central to fail-closed behavior, but the pack does not define key matching rules. Without explicit rules (case sensitivity, duplicate keys, allowed namespaces/prefixes), different adapters can interpret the same request differently.
- Required to be concrete:
  - Specify the extension key string format and comparison rules (case sensitivity and allowed characters)
  - Specify behavior for duplicate keys and conflicting values (if representable by the request type)
  - Specify whether unknown-key validation is performed before or after any key normalization
  - Specify whether backends may accept “aliases” for keys (and if not, state explicitly)
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-1-bh-c02-extension-allowlist-validator.md` L26-L40

### CA-0012 — `agent_kind` identity type is inconsistent (string vs AgentWrapperKind)
- Severity: major
- Category: consistency
- Location: `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md` L20-L25
- Excerpt: “Backend-provided: `agent_kind` string (for error reporting)”
- Problem: SEAM-2 calls for an `agent_kind` string, while SEAM-1 slices prefer referencing `AgentWrapperKind`. This affects error payload shape (`UnsupportedCapability(agent_kind, key)`) and test fixtures; implementers must decide which is canonical.
- Required to be concrete:
  - Choose the canonical identity type used across BH-C01/BH-C02 (string vs `AgentWrapperKind` or another type)
  - If a string is used, define its allowed values and formatting (e.g., `"codex"`, `"claude_code"`)
  - If `AgentWrapperKind` is used, define how it is rendered into error fields/messages (and ensure stable formatting)
  - Update all references in the pack so the same type is used consistently
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md` L39-L41

### CA-0013 — Bounded channel sizing is required but no concrete default/cfg point is specified
- Severity: major
- Category: operational
- Location: `docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md` L15-L15
- Excerpt: “Canonical bounded channel sizing guidance and behavior (at minimum: no unbounded buffering).”
- Problem: The pack requires bounded channels and mentions a “default and where it can be configured,” but does not specify an actual default capacity, configuration mechanism, or how it is tested. This affects backpressure behavior, memory usage, and correctness under load.
- Required to be concrete:
  - Specify the default bounded channel capacity (a concrete integer) used by the harness builder
  - Specify where/how it can be configured (if it is configurable) and the default when unspecified
  - Specify how the chosen capacity interacts with bounds enforcement and backpressure policy
  - Add at least one test (or documented invariant) that prevents accidental introduction of unbounded buffering
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-1-bh-c04-drain-while-polling-completion.md` L72-L81

### CA-0014 — Normalization scope references backend config defaults beyond env/timeout but does not define rules
- Severity: major
- Category: dependency
- Location: `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md` L25-L25
- Excerpt: “Backend config defaults (env, timeout, working_dir, etc.) as currently modeled.”
- Problem: The pack implies more config defaults participate in normalization (e.g., working_dir), but only env and timeout have any precedence rules. This leaves implementers to guess which fields should be normalized, potentially causing behavior changes during SEAM-5 migration.
- Required to be concrete:
  - Enumerate which backend config default fields are included in normalization (env, timeout, working_dir, ...)
  - For each included field, specify precedence rules (request vs backend defaults) and default/absent behavior
  - If some fields are explicitly excluded from normalization, state that and explain where they are handled instead
- Suggested evidence order: codebase → docs → external → decision

### CA-0015 — Normative dependencies (ADR-0013, DR-0012, universal specs) are referenced without concrete anchors or copied invariants
- Severity: major
- Category: dependency
- Location: `docs/project_management/packs/active/agent-api-backend-harness/scope_brief.md` L21-L27
- Excerpt: “MUST NOT change universal contract/spec semantics (see ADR-0013 “User Contract (Authoritative)”).”
- Problem: The pack repeatedly relies on external normative sources for semantics (ADR-0013, DR-0012, `docs/specs/universal-agent-api/*`), but does not pin exact sections/anchors or restate the required invariants in-place. This forces implementers to interpret external docs and increases the risk of implementing the wrong semantics while still “following the pack.”
- Required to be concrete:
  - Add concrete references (file + section headings or anchors) for ADR-0013 and DR-0012 semantics that this pack must preserve
  - Restate (briefly) the specific invariants that are relied on by BH-C02/BH-C03/BH-C04/BH-C05 (not just “see ADR”)
  - Identify which document is the local source of truth when there is drift (and how to resolve conflicts)
- Suggested evidence order: docs → codebase → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-4-completion-gating.md` L18-L20

### CA-0016 — Codex allowlisted extension keys are required for behavior-preserving migration but are not enumerated
- Severity: major
- Category: testing
- Location: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-5-backend-adoption-and-tests/slice-1-codex-backend-migration.md` L49-L54
- Excerpt: “the allowlisted extension keys for Codex (including Codex-specific exec policy keys)”
- Problem: SEAM-5 claims “no behavior change,” but the Codex allowlist is not listed. That makes it easy to accidentally omit an extension key during migration, changing behavior while still “meeting” the doc’s acceptance criteria.
- Required to be concrete:
  - Enumerate the full set of Codex allowlisted extension keys that BH-C02 must accept for `agent_kind == "codex"`
  - Define whether the allowlist is derived from a single source of truth (capabilities reporting) or duplicated across code paths
  - Add/point to a test that would fail if the allowlist changes unexpectedly during migration
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-5-backend-adoption-and-tests/slice-3-claude-backend-migration.md` L41-L45

### CA-0017 — `crate::bounds` enforcement is required, but the scope (events vs completion vs errors) and ordering are not pinned
- Severity: major
- Category: behavior
- Location: `docs/project_management/packs/active/agent-api-backend-harness/seam-5-backend-adoption-and-tests.md` L24-L27
- Excerpt: “Every forwarded event MUST pass through bounds enforcement and redaction rules.”
- Problem: Bounds enforcement is mandated in multiple seams, but the pack does not specify whether bounds also apply to completion payloads, error events, or error messages, nor whether bounds are applied before/after mapping and redaction. This affects correctness and security guarantees during migration.
- Required to be concrete:
  - Specify which payloads must be bounded/redacted (events, completion, error messages, and/or backend-specific payloads)
  - Specify the ordering: mapping → bounds → send, or bounds before mapping (and how to test it)
  - Specify whether bounds enforcement is applied at exactly one layer (harness) and how backends must avoid double-bounding
  - Add a pinned test that fails if a forwarded payload bypasses bounds enforcement
- Suggested evidence order: codebase → docs → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md` L31-L31

### CA-0018 — Cohesion audit artifact is stale relative to current pack state (can mislead implementers)
- Severity: minor
- Category: consistency
- Location: `docs/project_management/packs/active/agent-api-backend-harness/cohesion-audit.report.md` L87-L95
- Excerpt: “CH-0004 — SEAM-5 still marks harness test location as TBD...”
- Problem: The pack includes generated cohesion-audit outputs that now contradict the updated seam docs (e.g., it claims SEAM-5 test placement is still TBD). Keeping stale artifacts in an “active” pack directory reduces trust and creates ambiguity about what is current.
- Required to be concrete:
  - Label the cohesion audit outputs as historical snapshots with an explicit generated-at + “may be stale” disclaimer, or regenerate them after changes
  - Ensure generated audit outputs do not assert facts that are no longer true in the same directory
  - If audit artifacts are kept, define a lightweight rule for keeping them in sync with pack edits
- Suggested evidence order: docs → codebase → external → decision
- Cross-references:
  - `docs/project_management/packs/active/agent-api-backend-harness/cohesion-remediator.log.md` L97-L112

## Audited files
- docs/project_management/packs/active/agent-api-backend-harness/README.md
- docs/project_management/packs/active/agent-api-backend-harness/cohesion-audit.report.json
- docs/project_management/packs/active/agent-api-backend-harness/cohesion-audit.report.md
- docs/project_management/packs/active/agent-api-backend-harness/cohesion-audit.scan.after.json
- docs/project_management/packs/active/agent-api-backend-harness/cohesion-audit.scan.json
- docs/project_management/packs/active/agent-api-backend-harness/cohesion-remediator.log.md
- docs/project_management/packs/active/agent-api-backend-harness/scope_brief.md
- docs/project_management/packs/active/agent-api-backend-harness/seam-1-harness-contract.md
- docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md
- docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md
- docs/project_management/packs/active/agent-api-backend-harness/seam-4-completion-gating.md
- docs/project_management/packs/active/agent-api-backend-harness/seam-5-backend-adoption-and-tests.md
- docs/project_management/packs/active/agent-api-backend-harness/seam_map.md
- docs/project_management/packs/active/agent-api-backend-harness/threading.md
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
