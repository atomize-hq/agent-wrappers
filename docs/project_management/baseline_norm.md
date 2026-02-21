# Baseline Normalization: Tools Facet + `final_text` + Orthogonality CI Gates (conditional Codex `ItemFailed`)

## Summary
Implement baseline normalization across built-in backends (`codex`, `claude_code`) by:
- Standardizing tool lifecycle visibility via a universal **tools facet** stored in `AgentWrapperEvent.data` for `ToolCall`/`ToolResult`.
- Ensuring deterministic `completion.final_text` where extractable (Codex already; Claude Code will match deterministically).
- Enforcing orthogonality so **new `agent_api.*` capabilities require ≥2 built-in backends** (except allowlist), and capability matrix freshness is CI-gated.

**Codex `ItemFailed` policy (exact deterministic attribution rule)**
- Emit `ToolResult(phase="fail", status="failed")` for `ThreadEvent::ItemFailed` **only when**:
  - `item.extra["item_type"]` exists, **is a string**, and is in `{ "command_execution", "file_change", "mcp_tool_call", "web_search" }`.
- Otherwise, keep `ItemFailed → Error`.

---

## P0 — Repo hygiene (pre-step)
If there are uncommitted capability-matrix generator + docs changes, either:
- commit them first, **or**
- explicitly fold them into the same PR as S1–S4.

---

## S1 — Docs/contract: tools facet schema + capability ids + active Codex pack doc updates

### S1.T1 — Add “Tools facet (structured.v1)” to event envelope spec
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/docs/project_management/next/universal-agent-api/event-envelope-schema-spec.md`

**Add a normative section** defining `AgentWrapperEvent.data` when `kind ∈ {ToolCall, ToolResult}` and backend supports `agent_api.tools.structured.v1`:

**Schema**
```json
{
  "schema": "agent_api.tools.structured.v1",
  "tool": {
    "backend_item_id": "string|null",
    "thread_id": "string|null",
    "turn_id": "string|null",

    "kind": "string",
    "phase": "start|delta|complete|fail",
    "status": "pending|running|completed|failed|unknown",
    "exit_code": "integer|null",

    "bytes": { "stdout": "integer", "stderr": "integer", "diff": "integer", "result": "integer" },

    "tool_name": "string|null",
    "tool_use_id": "string|null"
  }
}
```

**Normative wording**
- `tool.kind` is an **open set**.
- `bytes.*` are integer counts; use **0 when absent/unknown**.
- `exit_code` is `integer|null`.
- Safety: metadata-only; MUST NOT include raw tool inputs/outputs, raw backend lines, diffs/patches, or tool payload JSON. Must obey existing 64KiB `data` bound.

**Recommended `tool.kind` values (non-normative)**
- Codex built-ins: `command_execution`, `file_change`, `mcp_tool_call`, `web_search`
- Claude built-ins: `tool_use`, `tool_result`

### S1.T2 — Document new capability ids + clarify semantics
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/docs/project_management/next/universal-agent-api/capabilities-schema-spec.md`

**Add ids**
- `agent_api.tools.structured.v1`
- `agent_api.tools.results.v1`
- `agent_api.artifacts.final_text.v1`

**Clarify semantics (normative text)**
- `agent_api.tools.results.v1`: backend can emit `ToolResult` for tool completions and tool failures **only when deterministically attributable** (not “every failure becomes ToolResult”).
- `agent_api.artifacts.final_text.v1`: backend can deterministically populate `final_text` when full assistant message text blocks are observed in the supported flow; `final_text=None` is valid otherwise.

### S1.T3 — Update “next” Codex adapter spec (feature-gated C1 doc)
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/docs/project_management/next/universal-agent-api/C1-spec.md`

**Update mapping section**
- For tool-ish items:
  - started/delta → `ToolCall`
  - completed → `ToolResult`
- Keep `ItemFailed → Error` as default, with additive carve-out:
  - emit `ToolResult(phase=fail,status=failed)` only under the exact rule at top.

### S1.T4 — Update active Codex pack contract mapping (authoritative for current Codex work)
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/docs/project_management/packs/active/agent-api-codex-stream-exec/contract.md`

**Update “Event mapping (normative)”**
- Replace the pinned “tool-ish item_type → ToolCall” single-line mapping with lifecycle-aware rules:
  - `item_type in {command_execution,file_change,mcp_tool_call,web_search}`:
    - `item.started`/`item.delta` → `AgentWrapperEventKind::ToolCall`
    - `item.completed` → `AgentWrapperEventKind::ToolResult`
  - `ItemFailed` default remains `Error`, plus conditional carve-out:
    - `item.failed` → `ToolResult(phase=fail,status=failed)` **only** when `item.extra["item_type"]` exists, is a string, and is in the tool-ish set above; otherwise `Error`.
- Keep existing stable payload/bounds/redaction requirements.

### S1.T5 — Update active Codex pack C1 spec out-of-scope contradiction
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/docs/project_management/packs/active/agent-api-codex-stream-exec/C1-spec.md`

**Revise “Out of scope”**
- Remove or narrow the bullet that forbids taxonomy improvements (the one mentioning emitting `ToolResult`).
- Replace with an updated, non-contradictory statement, e.g.:
  - out of scope: forcing payload schema parity across agents / emitting raw tool inputs/outputs
  - in scope: emitting `ToolResult` for tool completions and conditionally attributable tool failures, plus tools facet `data` (as part of baseline normalization)

---

## S2 — Codex backend: ToolResult for tool completion + structured tool facet `data` + conditional `ItemFailed`

### S2.T1 — Advertise new capabilities
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/backends/codex.rs`

Add to `capabilities()`:
- `agent_api.tools.structured.v1`
- `agent_api.tools.results.v1`
- `agent_api.artifacts.final_text.v1`

### S2.T2 — Mapping implementation (exact)
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/backends/codex.rs`

**Tool-ish mapping**
- `ItemStarted` tool-ish → `ToolCall` with tools facet `data`:
  - `phase="start"`, `status="running"`, `kind` per payload, bytes computed as lengths only.
- `ItemDelta` tool-ish → `ToolCall` with tools facet:
  - `phase="delta"`, `status="running"`, bytes computed as lengths only.
- `ItemCompleted` tool-ish → `ToolResult` with tools facet:
  - `phase="complete"`, `status="completed"`, bytes computed as lengths only.
- `ItemFailed`:
  - If `item.extra["item_type"]` matches tool-ish set (exact rule) → `ToolResult` with tools facet:
    - `phase="fail"`, `status="failed"`, `kind` derived from `item_type`, `bytes.*=0`.
  - Else → `Error` (current behavior).
- Keep transport/parse/normalize errors as `Error` (no change).

**Facet field rules**
- Never include raw content; only byte lengths.
- `backend_item_id=ItemFailure.item_id`, `thread_id/turn_id` from envelope, `exit_code` only if safe numeric and present (else null).

### S2.T3 — Tests (include required negative)
**Unit tests**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/backends/codex.rs`
  - `item_failed_without_item_type_maps_to_error_with_message`
  - `item_failed_with_tool_item_type_maps_to_tool_result_failed` (`item_type:"command_execution"`)
  - **Negative (required):** `item_failed_with_non_tool_item_type_maps_to_error` (`item_type:"agent_message"`)

**Fake Codex scenarios (quality tweak included)**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`
  - `tool_lifecycle_ok`: place sentinels only in stdout/stderr/diff/result fields of tool-ish items (so leak assertions are meaningful).
  - `tool_lifecycle_fail_unknown_type`: keep `item.failed.error.message` benign (no sentinel), since it is expected to surface via `Error.message`.
  - `tool_lifecycle_fail_known_type`: include `item_type:"command_execution"`; keep `error.message` benign.

**Integration tests**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/tests/c1_codex_stream_exec_adapter.rs`
  - `tool_lifecycle_ok`: assert `ToolCall` then `ToolResult`, both with tools facet `data.schema`, and no sentinel appears in `text/message/data`.
  - `tool_lifecycle_fail_unknown_type`: assert an `Error` event occurs.
  - `tool_lifecycle_fail_known_type`: assert a `ToolResult` occurs with `data.tool.status=="failed"`.

---

## S3 — Claude Code backend: deterministic `final_text` + tools facet on tool events (incl. deltas)

### S3.T1 — Advertise new capabilities
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/backends/claude_code.rs`

Add:
- `agent_api.tools.structured.v1`
- `agent_api.tools.results.v1`
- `agent_api.artifacts.final_text.v1`

### S3.T2 — Shared `final_text` truncation in `bounds`
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/bounds.rs`
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/backends/codex.rs`
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/backends/claude_code.rs`

Move Codex `enforce_final_text_bound` into `crate::bounds` and use from both backends.

### S3.T3 — Deterministic Claude `final_text` extraction
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/backends/claude_code.rs`

- Track `last_assistant_text` updated only on full `AssistantMessage` by joining `"text"` blocks with `\n`.
- Ignore deltas for `final_text`.
- `final_text=None` is acceptable when no full assistant message text blocks were observed.

### S3.T4 — Tools facet on Claude tool-ish wrapper events (incl. `input_json_delta`)
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/backends/claude_code.rs`

- Attach tools facet `data` on all `ToolCall`/`ToolResult`.
- `input_json_delta` → `ToolCall(phase="delta", status="running")` with `tool_use_id/tool_name` expected null in most cases (no inference/buffering).

### S3.T5 — Tests
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/tests/c1_claude_live_events.rs`

- Fake scenario emits assistant message text fixture; assert `completion.final_text` matches.
- Assert tool events have `data.schema == "agent_api.tools.structured.v1"` (including `input_json_delta`).

---

## S4 — Orthogonality enforcement: capability matrix freshness + “≥2 backends for `agent_api.*`”

### S4.T1 — Add `xtask capability-matrix-audit`
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/xtask/src/main.rs`
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/crates/xtask/src/capability_matrix.rs` (or a new module wired through `main.rs`)

Rule:
- For every `agent_api.*` id not in allowlist `{agent_api.run, agent_api.events, agent_api.events.live, agent_api.exec.non_interactive}`, require support by ≥2 built-in backends; else exit non-zero with a deterministic report.

### S4.T2 — CI job
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/.github/workflows/ci.yml`

Add job:
- `cargo run -p xtask -- capability-matrix`
- `git diff --exit-code docs/project_management/next/universal-agent-api/capability-matrix.md`
- `cargo run -p xtask -- capability-matrix-audit`

### S4.T3 — Charter note
**Touch**
- `/Users/spensermcconnell/__Active_Code/codex-wrapper/docs/project_management/next/cli-agent-onboarding-charter.md`

Add “Capability promotion rule” section referencing the audit + matrix freshness requirement.

---

## Regeneration + final validation
- Regenerate matrix: `cargo run -p xtask -- capability-matrix` (commit result)
- Validate: `cargo fmt --all -- --check` and `cargo test -p agent_api --all-features`
