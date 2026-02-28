# Threading — Universal Agent API session semantics (ADR-0015 + ADR-0017)

This section makes coupling explicit: contracts/interfaces, dependency edges, critical paths, and conflict-safe workstreams.

## Contract registry

- **Contract ID**: `SA-C01 typed id accessor helpers`
  - **Type**: API (library helpers)
  - **Owner seam**: SEAM-1
  - **Consumers (seams)**: SEAM-2 (and any future session-id consumers)
  - **Definition**:
    - `codex::ThreadEvent::thread_id() -> Option<&str>`
    - `claude_code::ClaudeStreamJsonEvent::session_id() -> Option<&str>`
  - **Versioning/compat**: additive; keep return types as `Option<&str>` so absence remains valid.

- **Contract ID**: `SA-C02 session handle facet (handle.v1)`
  - **Type**: schema/event
  - **Owner seam**: SEAM-2
  - **Consumers (seams)**: SEAM-3 (resume-by-id UX), external orchestrators
  - **Definition**: When a backend advertises `agent_api.session.handle.v1`, it emits:
    - exactly one early `Status` event whose `data` is the handle facet, and
    - `completion.data` containing the handle facet when a completion is produced and the id is known,
    per `docs/specs/universal-agent-api/event-envelope-schema-spec.md`.
  - **Versioning/compat**: stable `schema` string; facet-level `session.id` is opaque and backend-defined.

- **Contract ID**: `SA-C03 resume extension key (resume.v1)`
  - **Type**: config/schema (core extension key)
  - **Owner seam**: SEAM-3
  - **Consumers (seams)**: external orchestrators
  - **Definition**: `agent_api.session.resume.v1` object with selector `"last"` or `"id"` (closed schema), validated pre-spawn and mapped to backend resume surfaces per `docs/specs/universal-agent-api/extensions-spec.md`.
  - **Versioning/compat**: closed `.v1` schema; new semantics require a new versioned key.

- **Contract ID**: `SA-C04 fork extension key (fork.v1)`
  - **Type**: config/schema (core extension key)
  - **Owner seam**: SEAM-4
  - **Consumers (seams)**: external orchestrators
  - **Definition**: `agent_api.session.fork.v1` object with selector `"last"` or `"id"` (closed schema), validated pre-spawn and mapped to backend fork surfaces per `docs/specs/universal-agent-api/extensions-spec.md`.
  - **Versioning/compat**: closed `.v1` schema; new semantics require a new versioned key.

- **Contract ID**: `SA-C05 codex streaming resume (control + env overrides)`
  - **Type**: API (wrapper/library surface)
  - **Owner seam**: SEAM-3
  - **Consumers (seams)**: SEAM-3 (Codex `agent_api` backend mapping)
  - **Definition**: A Codex wrapper entrypoint for `codex exec resume` that preserves the invariants needed by `agent_api`:
    - per-run env overrides (merged `request.env`),
    - termination handle support (for explicit cancellation / `run_control`), and
    - a completion future that is consistent with existing exec streaming semantics.
  - **Versioning/compat**: internal; keep behavior parity with exec where possible.

- **Contract ID**: `SA-C06 codex app-server fork RPC surface`
  - **Type**: API (JSON-RPC method contract + notifications)
  - **Owner seam**: SEAM-4
  - **Consumers (seams)**: SEAM-4 (Codex fork mapping in `agent_api`)
  - **Definition**: A headless fork flow implemented via `codex app-server`:
    - identify fork source thread (for selector `"last"` likely via a discovery/listing method scoped to working dir),
    - fork via `thread/fork`,
    - prompt via `turn/start`,
    plus bounded mapping of notifications into Universal Agent API events.
  - **Versioning/compat**: pinned to the app-server protocol version used by `crates/codex::mcp`.

## Dependency graph (text)

- `SEAM-1 blocks SEAM-2` because: handle facet emission should source ids via typed accessors to avoid duplicated match logic in multiple crates.
- `SEAM-2 + SEAM-3 jointly unblock “resume-by-id UX”` because: callers need both (a) a stable id discovery surface (handle facet) and (b) a way to resume by id (selector `"id"`).
- `SEAM-4 (Codex fork) is blocked by SA-C06` because: a headless fork requires a pinned app-server RPC surface (`thread/fork` plus any required discovery) before `agent_api` can integrate it safely.

## Critical path

- Session handle facet (both backends): `SEAM-1 (accessors)` → `SEAM-2 (handle emission)`
- Resume-by-id UX end-to-end: `max(SEAM-3 (resume by id), SEAM-1 → SEAM-2 (id discovery))`
- Fork:
  - Claude can ship early once SEAM-4 Claude mapping + tests land.
  - Codex fork is gated by the app-server contract-definition work inside SEAM-4.

## Parallelization notes / conflict-safe workstreams

Because SEAM-2/3/4 all touch `crates/agent_api/src/backends/{codex,claude_code}.rs`, the safest parallelization is by **backend + crate** rather than by seam alone.

- **WS-A (Wrapper accessors)**: SEAM-1; touch surface:
  - `crates/codex/src/events.rs`
  - `crates/claude_code/src/stream_json.rs`
- **WS-B (Claude session semantics)**: Claude portions of SEAM-2/3/4; touch surface:
  - `crates/agent_api/src/backends/claude_code.rs`
  - `crates/agent_api/tests/**`
- **WS-C (Codex resume + handle)**: Codex portions of SEAM-2/3 plus SA-C05; touch surface:
  - `crates/agent_api/src/backends/codex.rs`
  - `crates/agent_api/src/backends/codex/mapping.rs`
  - `crates/codex/src/exec.rs`
  - `crates/codex/src/exec/streaming.rs`
  - `crates/agent_api/tests/**`
- **WS-D (Codex fork via app-server)**: Codex portion of SEAM-4 plus SA-C06; touch surface:
  - `crates/codex/src/mcp/protocol.rs`
  - `crates/codex/src/mcp/client.rs`
  - `crates/codex/src/mcp/tests_core/**`
  - `crates/agent_api/src/backends/codex.rs` (minimal wiring preferred; isolate logic in a new module if possible)
  - `crates/agent_api/tests/**`
- **WS-INT (Integration)**: lands WS-A, then merges WS-B/WS-C, then WS-D; runs the full suite and verifies behavior matches the canonical specs.

