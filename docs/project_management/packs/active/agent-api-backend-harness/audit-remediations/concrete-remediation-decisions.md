# Concrete Remediation Decisions (Backend Harness pack)

This file records explicit decisions introduced during concrete remediation where authoritative
evidence did not already pin an exact name/contract in the planning docs.

Date: 2026-02-23

## D1 — Canonical internal harness module + symbols

**Decision**: The internal harness module is `crates/agent_api/src/backend_harness.rs` and defines:

- `BackendHarnessAdapter` (`pub(crate)` trait)
- `BackendHarnessErrorPhase` (`pub(crate)` enum)
- `BackendSpawn` + `BackendDefaults` + `NormalizedRequest` (internal structs)
- `run_harnessed_backend(...) -> Result<AgentWrapperRunHandle, AgentWrapperError>` (harness entrypoint)

**Context**: `docs/adr/0013-agent-api-backend-harness.md` describes an internal harness but leaves the
module name and concrete symbol names as TBD. Downstream seams require stable identifiers for
searchability and cross-slice traceability.

**Chosen spec**: Names and signatures are pinned in:

- `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`

**Rationale**:

- Matches the file boundary suggested by ADR-0013 (“File/module boundaries (informative)”).
- Keeps the surface auditable and internal-only (no public API change).

**Implications**:

- Implementation MUST follow these names unless the pack is updated (rename is a spec change).

## D2 — Normalization scope in v1 excludes `working_dir` defaulting

**Decision**: SEAM-2 normalization consumes only:

- env defaults (`BackendDefaults.env`) and request env overrides, and
- timeout defaults (`BackendDefaults.default_timeout`) and request timeout override.

`working_dir` defaulting remains backend-owned in v1.

**Context**: The pack referenced backend config defaults beyond env/timeout (“working_dir, etc.”)
without specifying precedence rules, which blocked deterministic implementation.

**Chosen spec**: Pinned in:

- `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md`

**Rationale**:

- Avoids accidental behavior change while the harness is introduced (ADR-0013 “no behavior change” intent).
- Keeps SEAM-2 focused on the two invariants that are already consistently described and testable.

**Implications**:

- If the harness later centralizes `working_dir`, it requires a new explicit contract (new BH-C0x)
  and migration tests.

## D3 — Mapping hook is infallible

**Decision**: `BackendHarnessAdapter::map_event(...)` is infallible and returns `Vec<AgentWrapperEvent>`.
Parse failures MUST surface as `BackendError` at the typed stream boundary instead of as mapping failures.

**Context**: The pack required 0..N mapping but did not specify how to handle mapping failures.

**Chosen spec**: Pinned in:

- `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`
- `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-3-streaming-pump/slice-1-bh-c04-drain-while-polling-completion.md`

**Rationale**:

- Matches current backend implementations, which treat stream parse issues as stream errors, not mapping errors.
- Simplifies pump semantics and makes ordering/backpressure tests deterministic.

**Implications**:

- Backends must do any fallible parsing before calling `map_event`.

## D4 — Stream errors are non-fatal; completion outcome is authoritative

**Decision**:

- Typed stream `Err(BackendError)` produces a universal `Error` event and draining continues.
- Stream errors do not override the completion outcome.
- Completion `Err(BackendError)` determines `AgentWrapperRunHandle.completion` (gated by DR-0012).

**Context**: The pack implied error handling but did not pin the “winner” when both stream and
completion can fail.

**Chosen spec**: Pinned in:

- `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-1-harness-contract/slice-1-bh-c01-contract-definition.md`

**Rationale**:

- Matches current `agent_api` backend behavior patterns (Codex/Claude emit error events for stream
  issues while still producing a completion outcome).

**Implications**:

- Harness tests MUST pin this behavior (`completion_error_wins_over_stream_errors`).

## D5 — Driver task JoinHandles are detached; handle drop does not cancel draining

**Decision**: The harness spawns the pump/drainer and completion sender as detached `tokio::spawn(...)`
tasks and drops the `JoinHandle`s immediately. Dropping `AgentWrapperRunHandle` MUST NOT cancel draining.

**Context**: The pack required background tasks but did not define ownership/cancellation semantics,
risking premature cancellation or leaks.

**Chosen spec**: Pinned in:

- `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-4-completion-gating/slice-2-bh-c05-canonical-handle-builder.md`

**Rationale**:

- Preserves the BH-C04 drain-on-drop invariant and prevents accidental cancellation by value drop.
- Keeps lifetime ownership tied to the stream/future resources, not external handles.

**Implications**:

- Harness-level tests MUST detect regressions where receiver drop stops draining.

