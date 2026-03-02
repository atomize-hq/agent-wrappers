# Concrete Remediation Log

This log remediates findings from `concrete-audit.report.json` by patching the referenced documentation so it is executable without guesswork.

## Meta

- Repo: `codex-wrapper`
- Source report: `concrete-audit.report.json`
- Remediation started: 2026-02-23

## Triage (evidence-first buckets)

Buckets:

- **A. Local clarification** — the answer exists in the same doc pack; make it explicit.
- **B. Code-defined contract** — the repo’s code/tests already define the shape/behavior.
- **C. Doc-defined standard** — another internal doc/spec defines it; cross-link and copy invariants.
- **D. External standard** — behavior is constrained by an external spec/library; cite and pin.
- **E. Decision required** — insufficient evidence; make an explicit decision and log it.

| Issue | Severity | Primary bucket | Notes |
|---|---:|---|---|
| CA-0001 | blocker | B/E | Prefer reusing an existing harness/backend trait if present; otherwise decide. |
| CA-0002 | blocker | B/E | Prefer reusing existing request types; otherwise define `NormalizedRequest` schema. |
| CA-0003 | blocker | E | Requires a pinned error propagation model across stream vs completion. |
| CA-0004 | critical | C/E | Prefer existing repo error/redaction conventions; otherwise decide limits and rules. |
| CA-0005 | critical | A | Resolve inconsistency by stating one precedence table (env vs request vs defaults). |
| CA-0006 | critical | C/E | Pin `Duration` representation + enforcement rules in the harness. |
| CA-0007 | critical | D/E | Pin backpressure semantics (bounded channel + drop/block policy). |
| CA-0008 | critical | E | Pin mapping hook semantics (0..N events, ordering, mapping failure). |
| CA-0009 | critical | D/E | Pin driver task ownership/cancellation and drop semantics. |
| CA-0010 | major | E | Choose deterministic ordering for “first unknown key”. |
| CA-0011 | major | E | Pin extension key matching rules (case, namespaces, duplicates). |
| CA-0012 | major | B/A | Make `agent_kind` identity type consistent across the pack and with code. |
| CA-0013 | major | E | Choose default bounded channel size + configuration point. |
| CA-0014 | major | A/E | Make normalization scope explicit beyond env/timeout (or explicitly out-of-scope). |
| CA-0015 | major | C | Replace dangling references with concrete anchors and copied invariants. |
| CA-0016 | major | B | Enumerate Codex allowlisted extension keys from current behavior. |
| CA-0017 | major | B/E | Prefer existing `crate::bounds` contract; otherwise define ordering + scope. |
| CA-0018 | minor | A | Update or clearly label artifact as historical/non-normative. |

## Issue status

Each issue will be logged below with: evidence used, doc changes, and any decisions introduced.

---

### CA-0001 — BH-C01 harness adapter interface is described but not specified as an exact Rust API (Fixed)

Restated requirement: Pin the harness adapter interface and harness entrypoint as concrete Rust names + signatures (types, bounds, spawn/mapping hooks, and agent identity).

Evidence used:
- Universal core types available today: `crates/agent_api/src/lib.rs:37-163`.
- Harness intent + suggested file boundary: `docs/adr/0013-agent-api-backend-harness.md:96-123`.

Doc changes:
- Pinned canonical internal API (trait, structs, function signatures, bounds) in:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`
- Updated seam overview to reference the pinned names:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-1-harness-contract.md`

Decisions introduced:
- D1 (module + symbol names): `docs/decisions/concrete-remediation-decisions.md`.

---

### CA-0002 — NormalizedRequest is required, but its schema and adapter-policy flow are not defined (Fixed)

Restated requirement: Define the internal normalized request schema (field list + Rust types), what is preserved/discarded, how backend policy extraction works, and the normalization function signature + defaults.

Evidence used:
- Public request fields available today: `crates/agent_api/src/lib.rs:90-98`.
- Extensions gating rule: `docs/specs/universal-agent-api/extensions-spec.md:39-52`.
- Agent kind naming rules: `docs/specs/universal-agent-api/capabilities-schema-spec.md:11-22`.

Doc changes:
- Defined:
  - `NormalizedRequest<P>` schema,
  - exact normalization call order, and
  - explicit preservation/discard rules for `request.extensions`
  in `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md`.
- Cross-linked the same schema + policy flow in the BH-C01 contract definition:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`.

Decisions introduced:
- D2 (normalization scope): `docs/decisions/concrete-remediation-decisions.md`.

---

### CA-0003 — Backend stream/completion error propagation model is implied but not specified (Fixed)

Restated requirement: Pin what happens when the typed stream yields errors, when completion yields errors, which error “wins”, and how this interacts with DR-0012 gating and drain-on-drop.

Evidence used:
- Current Codex drain/forward loop (forward-flag + continue draining after receiver drop): `crates/agent_api/src/backends/codex.rs:429-469`.
- Current Claude drain/forward loop (same pattern): `crates/agent_api/src/backends/claude_code.rs:210-237`.
- DR-0012 completion finality rule: `docs/specs/universal-agent-api/run-protocol-spec.md:35-43`.

Doc changes:
- Pinned per-boundary behavior + “winner” rule in:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`
- Added an explicit required harness-level regression test spec:
  - `completion_error_wins_over_stream_errors` (documented in the same slice).

Decisions introduced:
- D4 (winner rule): `docs/decisions/concrete-remediation-decisions.md`.

---

### CA-0004 — Error mapping/redaction contract is described but missing concrete rules and payload limits (Fixed)

Restated requirement: Pin the error mapping API + phase enum, per-phase rules, redaction requirements, and bounds behavior.

Evidence used:
- Event envelope bounds + raw-line prohibition: `docs/specs/universal-agent-api/event-envelope-schema-spec.md:23-131`.
- Concrete bounds enforcement algorithms: `crates/agent_api/src/bounds.rs:1-99`.

Doc changes:
- Defined:
  - `BackendHarnessErrorPhase`,
  - `BackendHarnessAdapter::redact_error(...)`,
  - explicit redaction rules, and
  - “bounds enforced exactly once (harness-owned)” requirements
  in `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`.
- Added pinned test requirements for raw backend output non-leakage (backend-local tests) in the same slice.

---

### CA-0005 — BH-C03 timeout precedence is stated inconsistently (cannot implement deterministically) (Fixed)

Restated requirement: State timeout precedence unambiguously (including explicit “no timeout”), cover the 4 presence/absence combinations, and make all mentions consistent.

Evidence used:
- Current behavior in backends derives “request overrides default”:
  - Codex: `crates/agent_api/src/backends/codex.rs:347-351`
  - Claude: `crates/agent_api/src/backends/claude_code.rs:137-149`

Doc changes:
- Corrected and pinned the BH-C03 effective timeout rule in:
  - `docs/project_management/packs/active/agent-api-backend-harness/threading.md`
- Pinned the same effective timeout semantics in:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md`

---

### CA-0006 — Timeout derivation is specified, but timeout representation and enforcement behavior are not (Fixed)

Restated requirement: Pin the internal timeout type, what is timed, what error shape occurs, how “absent” vs explicit “no timeout” works, and require tests that lock the behavior.

Evidence used:
- `AgentWrapperRunRequest.timeout: Option<Duration>`: `crates/agent_api/src/lib.rs:90-98`.
- Codex wrapper treats `Duration::ZERO` as “no timeout”: `crates/codex/src/client_core.rs:84-112`.
- Existing timeout/drain regression in Codex backend: `crates/agent_api/src/backends/codex/tests.rs:220-292`.

Doc changes:
- Pinned:
  - `effective_timeout: Option<Duration>` representation,
  - explicit `Some(Duration::ZERO)` semantics,
  - “harness passes through; wrapper enforces” behavior, and
  - a concrete test-port requirement
  in `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md`.

---

### CA-0007 — Pump backpressure behavior is required but left as “define what happens under backpressure” (Fixed)

Restated requirement: Pin the backpressure algorithm and how it interacts with drain-on-drop, ordering, and bounds; require a regression test that fails on behavior changes.

Evidence used:
- Current pattern uses bounded `mpsc` + `send().await` + forward-flag:
  - Codex: `crates/agent_api/src/backends/codex.rs:439-463`
  - Claude: `crates/agent_api/src/backends/claude_code.rs:210-236`

Doc changes:
- Added a pinned algorithm (pseudo-code) and concrete required regression tests in:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-2-bh-c04-drain-on-drop-semantics.md`.

---

### CA-0008 — Mapping hook semantics (0..N events, ordering, bounds, mapping failures) are not pinned (Fixed)

Restated requirement: Pin mapping infallibility vs fallibility, ordering guarantees for 0..N, and bounds ordering.

Evidence used:
- Existing backends treat mapping as infallible and convert parse issues into stream errors:
  - Codex: `crates/agent_api/src/backends/codex.rs:448-456`
  - Claude: `crates/agent_api/src/backends/claude_code.rs:227-236`

Doc changes:
- Declared mapping as infallible by contract and pinned ordering rules in:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-1-bh-c04-drain-while-polling-completion.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-2-bh-c04-drain-on-drop-semantics.md`

Decisions introduced:
- D3 (mapping infallible): `docs/decisions/concrete-remediation-decisions.md`.

---

### CA-0009 — Harness driver task ownership/cancellation semantics are vague (risk of premature drop or leaks) (Fixed)

Restated requirement: Pin who owns tasks, cancellation behavior on handle drop vs events drop, and require tests that fail if tasks are dropped prematurely.

Evidence used:
- Minimum cancellation semantics (best-effort) in protocol: `docs/specs/universal-agent-api/run-protocol-spec.md:49-54`.
- Existing gating implementation anchor: `crates/agent_api/src/run_handle_gate.rs` (referenced by DR-0012 tests).

Doc changes:
- Pinned the harness driver split (pump/drainer + completion sender), detached JoinHandle policy, and drop semantics in:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-2-bh-c05-canonical-handle-builder.md`.

Decisions introduced:
- D5 (detached tasks; no cancellation on handle drop): `docs/decisions/concrete-remediation-decisions.md`.

---

### CA-0010 — Deterministic “first unknown key” selection ordering is not defined (Fixed)

Restated requirement: Pin ordering for “first unknown key”, stop-at-first vs aggregate, and key normalization rules; require a unit test.

Evidence used:
- `AgentWrapperRunRequest.extensions` uses `BTreeMap` (sorted keys): `crates/agent_api/src/lib.rs:90-98`.

Doc changes:
- Pinned lexicographic key selection (BTreeMap iteration order), stop-at-first, and exact-match rules in:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-1-bh-c02-extension-allowlist-validator.md`.

---

### CA-0011 — Extension key matching rules are not defined (case sensitivity, duplicates, namespaces) (Fixed)

Restated requirement: Pin extension key comparison rules and namespace rules.

Evidence used:
- Extension keys must be capability ids (same string): `docs/specs/universal-agent-api/capabilities-schema-spec.md:93-97`.
- Ownership + fail-closed validation: `docs/specs/universal-agent-api/extensions-spec.md:39-59`.

Doc changes:
- Pinned exact-match, case-sensitive, no-alias rules and namespace ownership in:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md`.

---

### CA-0012 — `agent_kind` identity type is inconsistent (string vs AgentWrapperKind) (Fixed)

Restated requirement: Choose and use a single canonical type for agent identity and define formatting for errors.

Evidence used:
- Canonical type: `AgentWrapperKind` in `crates/agent_api/src/lib.rs:37-54`.
- Naming constraints + reserved ids: `docs/specs/universal-agent-api/capabilities-schema-spec.md:11-22`.

Doc changes:
- Made `agent_kind: AgentWrapperKind` canonical in normalization inputs and pinned `.as_str()` rendering rule for errors in:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`

---

### CA-0013 — Bounded channel sizing required but no default/cfg point specified (Fixed)

Restated requirement: Pin the default bounded channel capacity and whether/how it can be configured.

Evidence used:
- Both built-in backends use `mpsc::channel::<AgentWrapperEvent>(32)`:
  - Codex: `crates/agent_api/src/backends/codex.rs:277-279`
  - Claude: `crates/agent_api/src/backends/claude_code.rs:116-118`

Doc changes:
- Pinned `DEFAULT_EVENT_CHANNEL_CAPACITY = 32` and “not configurable in v1” in:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-3-streaming-pump.md`.

Decisions introduced:
- D1 (symbol) and D5 (task behavior): `docs/decisions/concrete-remediation-decisions.md`.

---

### CA-0014 — Normalization scope references defaults beyond env/timeout but does not define rules (Fixed)

Restated requirement: Enumerate which backend defaults are normalized and explicitly exclude others with ownership.

Evidence used:
- The backends’ defaults model includes many fields; only env + timeout are universally comparable today:
  - Codex effective timeout derivation: `crates/agent_api/src/backends/codex.rs:347-351`
  - Claude effective timeout derivation: `crates/agent_api/src/backends/claude_code.rs:137-149`

Doc changes:
- Explicitly scoped normalization to env + timeout only, and excluded `working_dir` defaulting in v1 in:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`.

Decisions introduced:
- D2: `docs/decisions/concrete-remediation-decisions.md`.

---

### CA-0015 — Normative dependencies are referenced without concrete anchors or copied invariants (Fixed)

Restated requirement: Add concrete anchors for ADR/spec dependencies and restate relied-on invariants.

Evidence used:
- ADR harness “User Contract (Authoritative)” lists the preserved invariants: `docs/adr/0013-agent-api-backend-harness.md:71-90`.
- Specs define the actual normative contracts:
  - `docs/specs/universal-agent-api/*`.

Doc changes:
- Added explicit dependency anchors and a pinned “specs win on drift” rule in:
  - `docs/project_management/packs/active/agent-api-backend-harness/scope_brief.md`.

---

### CA-0016 — Codex allowlisted extension keys required but not enumerated (Fixed)

Restated requirement: Enumerate Codex extension allowlist keys and define source-of-truth + test expectations.

Evidence used:
- Current Codex backend allowlist constants:
  - `crates/agent_api/src/backends/codex.rs` (`EXT_NON_INTERACTIVE`, `EXT_CODEX_APPROVAL_POLICY`, `EXT_CODEX_SANDBOX_MODE`).
- Core key schema + default: `docs/specs/universal-agent-api/extensions-spec.md:67-90`.

Doc changes:
- Enumerated allowlisted keys and pinned “capabilities is the single source of truth” in:
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-5-backend-adoption-and-tests/slice-1-codex-backend-migration.md`.

---

### CA-0017 — `crate::bounds` enforcement required but scope + ordering not pinned (Fixed)

Restated requirement: Pin exactly what is bounded, ordering, single-layer enforcement, and require a regression test.

Evidence used:
- Bounds algorithms and constants: `crates/agent_api/src/bounds.rs:1-99`.
- Envelope bounds contract: `docs/specs/universal-agent-api/event-envelope-schema-spec.md:23-112`.

Doc changes:
- Pinned bounds scope + ordering and “exactly once in harness” rule in:
  - `docs/project_management/packs/active/agent-api-backend-harness/seam-5-backend-adoption-and-tests.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`
  - `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-2-bh-c04-drain-on-drop-semantics.md` (added a concrete regression test spec).

---

### CA-0018 — Cohesion audit artifact is stale relative to current pack state (can mislead implementers) (Fixed)

Restated requirement: Label cohesion audit artifacts as snapshots and define a lightweight sync rule.

Evidence used:
- Cohesion audit artifacts are committed in-pack alongside evolving docs.

Doc changes:
- Added a pinned “snapshot disclaimer” and sync rule in:
  - `docs/project_management/packs/active/agent-api-backend-harness/cohesion-audit.report.md`.
