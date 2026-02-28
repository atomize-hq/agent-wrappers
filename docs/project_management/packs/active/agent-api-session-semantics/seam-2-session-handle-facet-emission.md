# SEAM-2 — Session handle facet emission (`agent_api.session.handle.v1`) (uaa-0015)

- **Name**: Stable session/thread id facet emission (events + completion)
- **Type**: integration
- **Goal / user value**: Make “resume-by-id” ergonomic by surfacing the backend-defined session/thread id in a stable, bounded metadata facet, without changing the public `agent_api` Rust type shapes.
- **Scope**
  - In:
    - Implement capability id `agent_api.session.handle.v1` in built-in `agent_api` backends:
      - Capture the id from typed parsed backend events (Codex `thread_id`, Claude `session_id`).
      - Emit **exactly one** early `AgentWrapperEventKind::Status` event carrying:
        - `data = { "schema": "agent_api.session.handle.v1", "session": { "id": "<opaque>" } }`
      - Attach the same facet to `AgentWrapperCompletion.data` whenever a completion is produced and the id is known.
    - Enforce facet-level bounds:
      - `len(session.id) <= 1024` bytes (UTF-8) or else omit (MUST NOT truncate) and emit a safe warning `Status`.
    - Advertise `agent_api.session.handle.v1` in `AgentWrapperCapabilities.ids` only once the above is implemented and tested.
    - Add tests pinning placement rules and bounds behavior for both backends.
  - Out:
    - Any spec changes (authoritative rules are already in `event-envelope-schema-spec.md`).
    - Any new required fields on `AgentWrapperEvent` or `AgentWrapperCompletion`.
- **Primary interfaces (contracts)**
  - Inputs:
    - Typed backend events and their id fields (via SEAM-1 accessors).
    - Universal event/completion bounds enforcement (`crates/agent_api/src/bounds.rs`).
  - Outputs:
    - Event stream: one early `Status` event containing the handle facet in `data`.
    - Completion: `AgentWrapperCompletion.data = Some(handle_facet)` when id known.
    - Capability advertisement: `agent_api.session.handle.v1` present in capabilities only when behavior is implemented.
- **Key invariants / rules**:
  - Emit the facet only for backends that advertise `agent_api.session.handle.v1`.
  - The facet is metadata-only and MUST NOT include raw backend lines.
  - If the facet would violate bounds (oversize id), omit (do not truncate) and keep the run safe.
- **Dependencies**
  - Blocks:
    - None (this is a leaf capability once accessors exist).
  - Blocked by:
    - **SEAM-1** (typed id accessors) — use accessor helpers for extraction to avoid duplication.
- **Touch surface**:
  - `crates/agent_api/src/backends/codex.rs`
  - `crates/agent_api/src/backends/codex/mapping.rs`
  - `crates/agent_api/src/backends/claude_code.rs`
  - `crates/agent_api/tests/**` (new tests for handle facet behavior)
  - (Optional) `crates/agent_api/src/bounds.rs` (helpers for facet-level id bound enforcement)
- **Verification**:
  - Tests prove for each built-in backend when capability is advertised:
    - an early `Status` event includes the facet **exactly once**,
    - `completion.data` includes the facet when completion exists and id was observed, and
    - oversize ids are omitted (not truncated) and cause only safe/bounded warnings.
- **Risks / unknowns**
  - Risk: emitting a synthetic `Status` event changes event ordering expectations.
  - De-risk plan: prefer attaching the facet to an existing early `Status` event when feasible (Codex `thread.started`, Claude `system init`), falling back to a synthetic event only when needed.
- **Rollout / safety**:
  - Ship capability-gated: do not advertise `agent_api.session.handle.v1` until passing tests for both event and completion attachment.

## Downstream decomposition prompt

Decompose into: (1) per-backend id capture + run-local storage, (2) event emission “exactly once” implementation, (3) completion attachment, (4) bounds + oversize behavior, (5) regression tests for both backends.

