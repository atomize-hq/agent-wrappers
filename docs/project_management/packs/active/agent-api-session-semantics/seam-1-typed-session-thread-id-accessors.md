# SEAM-1 — Typed session/thread id accessors (uaa-0017)

- **Name**: Typed id accessors on backend event models (Codex + Claude Code)
- **Type**: integration
- **Goal / user value**: Provide tiny, reusable accessors so “what is the current session/thread id?” can be answered without duplicating per-variant match logic across `wrapper_events`, `agent_api`, and future consumers.
- **Scope**
  - In:
    - Codex: add `codex::ThreadEvent::thread_id() -> Option<&str>` returning the best-effort thread id when the variant contains one.
    - Claude Code: add `claude_code::ClaudeStreamJsonEvent::session_id() -> Option<&str>` returning the best-effort session id (including `Unknown { session_id: Some(...) }`).
    - Add tests proving the accessors cover the expected variants and do not allocate.
    - Adopt the accessors in `crates/wrapper_events` so session/thread id extraction match logic is not duplicated:
      - `crates/wrapper_events/src/codex_adapter.rs`
      - `crates/wrapper_events/src/claude_code_adapter.rs`
  - Out:
    - Any Universal Agent API spec changes (this seam is code-only ergonomics).
    - Any new public “session handle” surfaces in `agent_api` (that is SEAM-2).
- **Primary interfaces (contracts)**
  - Inputs:
    - Typed backend events:
      - `codex::ThreadEvent`
      - `claude_code::ClaudeStreamJsonEvent`
  - Outputs:
    - `ThreadEvent::thread_id() -> Option<&str>`
    - `ClaudeStreamJsonEvent::session_id() -> Option<&str>`
- **Key invariants / rules**:
  - Accessors are best-effort and must return `None` when the event carries no id (e.g., Codex `ThreadEvent::Error`).
  - Accessors MUST NOT parse raw stdout/stderr lines; they only read already-typed fields.
- **Dependencies**
  - Blocks:
    - **SEAM-2** (handle facet emission) — SEAM-2 should source ids via these accessors to avoid duplicated match logic.
  - Blocked by:
    - None.
- **Touch surface**:
  - Codex:
    - `crates/codex/src/events.rs`
    - `crates/codex/tests/**` (new or adjusted tests)
  - Claude Code:
    - `crates/claude_code/src/stream_json.rs`
    - `crates/claude_code/tests/**` (new or adjusted tests)
  - wrapper_events:
    - `crates/wrapper_events/src/codex_adapter.rs`
    - `crates/wrapper_events/src/claude_code_adapter.rs`
- **Verification**:
  - Unit tests demonstrate:
    - `thread_id()` returns `Some(...)` for `ThreadStarted` and other id-bearing variants.
    - `session_id()` returns `Some(...)` for known variants and `Unknown { session_id: Some(...) }`.
    - Returned references point into the event (no allocation required).
  - wrapper_events no longer duplicates per-variant match logic for session/thread ids; adapters call the accessors.
- **Risks / unknowns**
  - Risk: downstream code relies on accessors as “always present”.
  - De-risk plan: keep return types as `Option<&str>` and document that absence is valid.
- **Rollout / safety**:
  - Additive API; ship behind normal crate versioning (no behavior changes without consumers updating call sites).

## Downstream decomposition prompt

Decompose into two tiny slices: (1) implement + test `ThreadEvent::thread_id()`, (2) implement + test `ClaudeStreamJsonEvent::session_id()`. Then update downstream call sites (wrapper_events and SEAM-2) to use these helpers.
