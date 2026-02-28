# Scope Brief — Universal Agent API session semantics (ADR-0015 + ADR-0017)

- **Goal (1 sentence)**: Ship universal, capability-gated session/thread semantics for `agent_api` (resume/fork + discoverable session ids) so orchestration code can implement “resume last / resume by id / fork” without per-backend parsing and without breaking Universal Agent API safety/bounds rules.
- **Why now**: The core spec contracts for session extension keys and session-handle emission are already pinned under `docs/specs/universal-agent-api/**`; remaining work is to implement and advertise them across built-in backends so the features become usable in real orchestrators.
- **Primary user(s) + JTBD**:
  - Orchestrators/hosts using `crates/agent_api`: “Resume or fork a conversation deterministically across Codex and Claude Code.”
  - Maintainers onboarding new backends: “Implement session semantics once via stable keys + facets, not bespoke glue per backend.”
- **In-scope**:
  - Implement core extension keys in `crates/agent_api` built-in backends:
    - `agent_api.session.resume.v1` (selectors: `"last"` and `"id"`)
    - `agent_api.session.fork.v1` (selectors: `"last"` and `"id"`)
  - Implement session handle facet emission (`agent_api.session.handle.v1`) in `AgentWrapperEvent.data` (early `Status`) and `AgentWrapperCompletion.data` (when known).
  - Add typed accessor helpers on wrapper event models so session/thread id extraction is implemented once per backend type and reused by both `crates/wrapper_events` and `agent_api` (per `docs/backlog.json`: uaa-0017).
  - Add test coverage pinning: validation timing, mutual exclusivity, “exactly one” handle emission, completion attachment, and bounds/redaction posture.
- **Out-of-scope**:
  - A universal session listing/inspection API.
  - Standardizing backend session id formats (ids remain opaque backend-defined strings).
  - Changing the public `agent_api` Rust contract shape beyond what is already specified (this pack assumes the specs are authoritative).
  - Implementing interactive/TUI session flows (e.g., relying on `codex fork` if it is not a safe headless + streaming surface).
- **Success criteria**:
  - Backends that support session semantics advertise the keys/capabilities and behave per the canonical specs.
  - A caller can:
    - resume “last” and “by id” via `AgentWrapperRunRequest.extensions`, and
    - observe the backend session/thread id via the stable handle facet (events + completion),
    without backend-specific parsing.
  - Regression tests exist for both built-in backends covering the positive path and at least one failure/edge case per capability.
- **Constraints**:
  - MUST keep fail-closed capability gating for extension keys (reject unknown keys pre-spawn).
  - MUST keep session keys mutually exclusive (`resume.v1` XOR `fork.v1`) per `extensions-spec.md`.
  - MUST keep bounds + safety posture: no raw backend lines in `AgentWrapperEvent.data` / `AgentWrapperCompletion.data`; enforce event/completion `data` size limits and the `session.id <= 1024` bound (omit, do not truncate).
  - Universal run contract still requires a non-empty prompt; session semantics are “resume/fork + send follow-up prompt”, not “resume with no prompt”.
- **External systems / dependencies**:
  - Codex CLI:
    - `codex exec resume` (streaming JSONL)
    - `codex app-server` (JSON-RPC thread/turn management) for fork semantics (per ADR-0015).
  - Claude Code CLI: `claude --print --continue/--resume/--fork-session` with stream-json output.
  - Parser contracts used to safely extract ids:
    - `docs/specs/codex-thread-event-jsonl-parser-contract.md`
    - `docs/specs/claude-stream-json-parser-contract.md`
- **Known unknowns / risks**:
  - Codex fork mapping: confirm the app-server protocol supports `thread/fork` (and any required discovery such as `thread/list`) and implement typed wrappers if missing.
  - Codex resume mapping: `crates/codex` currently exposes streaming resume without a termination handle and without per-run env overrides; `agent_api` will likely need a control/override-capable variant to preserve cancellation + env semantics parity with `exec`.
  - Ensuring the handle facet is emitted **exactly once** per run and attached to completion when known, without violating bounds (omit-on-oversize + safe warning).
- **Assumptions**:
  - Canonical specs under `docs/specs/universal-agent-api/**` already register:
    - the session extension keys (`agent_api.session.resume.v1`, `agent_api.session.fork.v1`), and
    - the session handle capability/facet (`agent_api.session.handle.v1`),
    and this pack is purely the execution plan for implementations/tests.
  - Codex `thread_id` and Claude `session_id` can be extracted from typed parsed events (no raw-line parsing).

## Backlog anchors (order-to-ship)

From `docs/backlog.json` (in dependency/order-to-ship sequence, with current status) and the ADR that defines it:

- `uaa-0013` (done) → ADR-0015 `docs/adr/0015-universal-agent-api-session-extensions.md`
- `uaa-0011` (done; redundant design item) → ADR-0015 `docs/adr/0015-universal-agent-api-session-extensions.md`
- `uaa-0004` (todo) → ADR-0015 `docs/adr/0015-universal-agent-api-session-extensions.md`
- `uaa-0005` (todo) → ADR-0015 `docs/adr/0015-universal-agent-api-session-extensions.md`
- `uaa-0007` (todo) → ADR-0015 `docs/adr/0015-universal-agent-api-session-extensions.md`
- `uaa-0017` (todo; prerequisite for uaa-0015) → ADR-0017 `docs/adr/0017-universal-agent-api-session-thread-id-surfacing.md`
- `uaa-0015` (todo) → ADR-0017 `docs/adr/0017-universal-agent-api-session-thread-id-surfacing.md`

## Capability inventory (implied by scope; no seams yet)

- Session selection via core extension keys (schema + closed validation + mutual exclusivity).
- Backend mapping for:
  - Codex: `exec resume` streaming, plus a headless fork surface (ADR-0015 recommends app-server).
  - Claude Code: `--continue`, `--resume <id>`, `--fork-session` (stream-json output).
- Typed id extraction primitives:
  - Codex thread id (`ThreadEvent::ThreadStarted.thread_id` and friends).
  - Claude session id (`ClaudeStreamJsonEvent::SystemInit.session_id` and friends).
- Session handle facet emission (event + completion) behind capability id `agent_api.session.handle.v1`.
- Capability advertisement updates (only advertise once implemented and tested).
- Capability matrix documentation handoff: after advertising new session capability ids, regenerate and commit `docs/specs/universal-agent-api/capability-matrix.md` via `cargo run -p xtask -- capability-matrix`.
- Tests:
  - request validation failures (type errors, unknown keys, selector/id rules, mutual exclusivity),
  - per-backend spawn mapping (argv / command shape),
  - handle facet placement + bounds enforcement + “exactly one” emission.

## Canonical dependencies (authoritative anchors)

- Extension key registry + validation rules (normative):
  - `docs/specs/universal-agent-api/extensions-spec.md`
- Capability ids + bucketing rules (normative):
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`
- Event + completion envelope + handle facet schema/emission (normative):
  - `docs/specs/universal-agent-api/event-envelope-schema-spec.md`
- Run protocol validation timing + fail-closed rules (normative):
  - `docs/specs/universal-agent-api/run-protocol-spec.md`

Conflict resolution rule (pinned):
- If this pack drifts from a document under `docs/specs/universal-agent-api/`, the spec wins.
- This pack MUST be updated to match the spec before implementation proceeds.
