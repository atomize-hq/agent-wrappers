### S1 — Canonicalize CA-C01 across pack + specs

- **User/system value**: A single, unambiguous source of truth for explicit cancellation (capability id, API shape, and completion semantics) so downstream seams can implement without re-litigating details.
- **Scope (in/out)**:
  - In:
    - Make SEAM-1’s contract fully concrete (remove placeholders like `-> ...`).
    - Ensure canonical spec documents match the pack contract and DR selections.
  - Out:
    - Any harness/driver implementation details (SEAM-2).
    - Any backend process termination implementation details (SEAM-3).
- **Acceptance criteria**:
  - The capability id is exactly `agent_api.control.cancel.v1` everywhere.
  - The cancellation completion outcome is pinned everywhere:
    - `Err(AgentWrapperError::Backend { message })` where `message == "cancelled"`.
  - The API surface is pinned everywhere:
    - `AgentWrapperGateway::run_control(...) -> Future<Output = Result<AgentWrapperRunControl, AgentWrapperError>>`
    - `AgentWrapperRunControl { handle: AgentWrapperRunHandle, cancel: AgentWrapperCancelHandle }`
    - `AgentWrapperCancelHandle::cancel(&self)` is idempotent and best-effort.
  - Pack-local contract and canonical specs do not contradict each other.
- **Dependencies**:
  - None (critical-path start).
- **Verification**:
  - Grep-based consistency check (strings + capability id).
  - Review that `docs/specs/universal-agent-api/*` documents agree with the pack contract.
- **Rollout/safety**:
  - Purely doc-level; no runtime behavior changes.

#### S1.T1 — Make the pack seam contract fully concrete

- **Outcome**: `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md` contains exact signatures/types (no ellipses) and matches the DR selections.
- **Inputs/outputs**:
  - Input: `docs/project_management/packs/active/agent-api-explicit-cancellation/seam-1-cancellation-contract.md`
  - Output: updated pack seam contract with concrete signatures + pinned strings.
- **Implementation notes**:
  - Inline the exact signature shape used in the canonical contract (`Pin<Box<dyn Future<...>>>` style).
  - Ensure the error shape remains DR-CA-0002 Option A (stringly-typed `"cancelled"`).
- **Acceptance criteria**:
  - The pack contract is independently implementable without consulting other docs.
- **Test notes**:
  - N/A (doc-only).
- **Risk/rollback notes**:
  - Low risk; rollback by reverting the doc change.

Checklist:
- Implement: replace `-> ...` and any other placeholders with concrete signatures/types.
- Validate: confirm capability id + pinned strings match `threading.md` + DR.
- Cleanup: ensure wording does not drift from canonical specs.

#### S1.T2 — Align canonical contract surface (`contract.md`) with CA-C01

- **Outcome**: `docs/specs/universal-agent-api/contract.md` is the canonical, concrete Rust surface for `run_control(...)`, `AgentWrapperRunControl`, and `AgentWrapperCancelHandle`.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/contract.md`
  - Output: updated canonical contract (if needed) matching CA-C01.
- **Implementation notes**:
  - Keep `AgentWrapperCancelHandle` representation private (opaque), but public API concrete.
  - Ensure doc comments explicitly call out idempotence and best-effort semantics.
- **Acceptance criteria**:
  - The contract surface matches the pack seam contract exactly.
- **Test notes**:
  - N/A (doc-only).
- **Risk/rollback notes**:
  - Low risk; rollback by reverting the doc change.

Checklist:
- Implement: update types/signatures/comments as needed.
- Validate: confirm all identifiers match the crate naming (`agent_api`).
- Cleanup: cross-link to `run-protocol-spec.md` for semantics.

#### S1.T3 — Align run protocol semantics (`run-protocol-spec.md`) with CA-C01

- **Outcome**: `docs/specs/universal-agent-api/run-protocol-spec.md` contains the normative explicit cancellation semantics (capability gating, idempotence, pinned completion error outcome).
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/run-protocol-spec.md`
  - Output: updated protocol spec (if needed) matching CA-C01.
- **Implementation notes**:
  - Ensure “no late events after completion” and drain-on-drop invariants are explicitly preserved.
  - Keep explicit cancellation separate from drop semantics (“best-effort” remains for drop).
- **Acceptance criteria**:
  - Protocol semantics match the pack contract’s pinned strings and error shape exactly.
- **Test notes**:
  - N/A (doc-only).
- **Risk/rollback notes**:
  - Low risk; rollback by reverting the doc change.

Checklist:
- Implement: update explicit cancellation section as needed.
- Validate: confirm pinned error message uses `"cancelled"` exactly.
- Cleanup: ensure section headings/wording remain normative (RFC 2119 keywords).

#### S1.T4 — Align capability meaning (`capabilities-schema-spec.md`) with CA-C01

- **Outcome**: `docs/specs/universal-agent-api/capabilities-schema-spec.md` defines `agent_api.control.cancel.v1` and its minimum semantics without ambiguity.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/capabilities-schema-spec.md`
  - Output: updated capability spec (if needed) matching CA-C01.
- **Implementation notes**:
  - Explicitly tie the capability meaning to `run_control(...)` + `cancel()` per `run-protocol-spec.md`.
- **Acceptance criteria**:
  - Capability id meaning is consistent with the contract and run protocol specs.
- **Test notes**:
  - N/A (doc-only).
- **Risk/rollback notes**:
  - Low risk; rollback by reverting the doc change.

Checklist:
- Implement: update/confirm `agent_api.control.cancel.v1` entry.
- Validate: confirm naming and cross-links.
- Cleanup: ensure it remains in the “Standard capability ids” section.

