# S2 — Shared request validation (safe/redacted) + validate-before-hook

- **User/system value**: Makes invalid requests fail fast and deterministically, without spawning or even invoking backend hook code, while guaranteeing safe/redacted `InvalidRequest` messages.
- **Scope (in/out)**:
  - In:
    - Implement SEAM-1-owned normalization/validation helpers for:
      - server names (trim + non-empty),
      - `Stdio` transport (`command` non-empty; trim + non-empty items; `argv = command + args`),
      - `Url` transport (`http`/`https` absolute URL parsing; optional env var name regex).
    - Ensure all validation failures are `AgentWrapperError::InvalidRequest { message }` with safe messages that **do not** echo raw user-provided values.
    - Wire gateway entrypoints to validate **after** backend resolution + capability gating and **before** invoking backend hooks:
      - resolve backend → capability check → validate/normalize → invoke hook.
    - Provide crate-level helpers that SEAM-3/4 backend hook implementations can reuse before any process spawn.
  - Out:
    - Backend mapping of validated requests to upstream argv (SEAM-3/4).
    - Backend config precedence + environment merging policy implementation (SEAM-2/3/4).
    - Cross-backend conformance tests beyond validation unit coverage (SEAM-5).
- **Acceptance criteria**:
  - Validation behavior matches the canonical spec (`docs/specs/universal-agent-api/mcp-management-spec.md`) for:
    - trimmed/non-empty names,
    - `Stdio` and `Url` transport field validation rules,
    - `bearer_token_env_var` regex `^[A-Za-z_][A-Za-z0-9_]*$`.
  - Gateway never invokes backend hooks for invalid requests (even if the backend advertises the capability).
  - Validation errors are safe/redacted (no raw `name`, `url`, `command`, `args`, or env var values in messages).
  - Deterministic behavior is pinned by unit tests.
- **Dependencies**:
  - Surface from S1: `agent_api::mcp` types + gateway/hook entrypoints + capability ids.
- **Verification**:
  - `cargo test -p agent_api --features codex,claude_code`

## Atomic Tasks

#### S2.T1 — Implement shared request normalization + validation helpers

- **Outcome**: SEAM-1 owns a single validation implementation that can be called by:
  - gateway entrypoints (this seam), and
  - SEAM-3/4 backend hook implementations (before spawn),
  to prevent semantic drift.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Server name validation” + “Transport field validation”)
  - Output: `crates/agent_api/src/mcp.rs` (or a `mcp::validation` submodule) with `pub(crate)` helper fns, such as:
    - `normalize_server_name(...) -> Result<String, AgentWrapperError>`
    - `normalize_add_transport(...) -> Result<AgentWrapperMcpAddTransport, AgentWrapperError>`
- **Implementation notes**:
  - Prefer helpers that return normalized values (trimmed strings), so backends can safely construct argv without repeating trim logic.
  - Keep `InvalidRequest` messages stable + operator-safe (field-oriented, not value-oriented).
- **Acceptance criteria**:
  - Helpers cover all pinned rules and return only `InvalidRequest` on validation failures.
- **Test notes**:
  - Unit tests in S2.T3 should call helpers directly (table-driven) to keep failures local.
- **Risk/rollback notes**: internal-only helpers; safe.

Checklist:
- Implement: normalization helpers for name + both transport variants.
- Test: direct helper unit tests.
- Validate: no error message contains raw user-provided strings.
- Cleanup: keep helpers small and reusable.

#### S2.T2 — Wire gateway to validate/normalize before invoking backend hooks

- **Outcome**: Gateway behavior is deterministic and enforces “validate before spawn” by construction:
  - resolve backend → capability check → validate/normalize → invoke hook.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/lib.rs` (gateway impl)
- **Implementation notes**:
  - Pin the ordering explicitly in code comments (and in the spec if clarification is needed while the spec is Draft).
  - Pass normalized requests to backend hooks (e.g., trimmed `name`, normalized transport fields).
- **Acceptance criteria**:
  - Invalid requests return `Err(InvalidRequest { .. })` and backend hooks are not invoked.
- **Test notes**:
  - S2.T4 adds a backend that would record invocations; the count must remain 0 on invalid requests.
- **Risk/rollback notes**: behavior change is new API only (no compatibility surface yet).

Checklist:
- Implement: gateway validation + request normalization.
- Test: gateway-level tests (S2.T4).
- Validate: ordering is resolve → gate → validate → hook.
- Cleanup: avoid duplicated validation in each gateway method (factor shared code if needed).

#### S2.T3 — Add validation unit tests (rules + redaction)

- **Outcome**: Deterministic unit coverage for validation rules, including redaction.
- **Inputs/outputs**:
  - Output: tests in `crates/agent_api/src/mcp.rs` (or `crates/agent_api/src/lib.rs`)
- **Implementation notes**:
  - Cover at minimum:
    - name trimming + empty rejection,
    - `Stdio.command` empty rejection,
    - whitespace-only items in `command` / `args`,
    - `Url.url` empty rejection,
    - non-absolute or non-http(s) URL rejection,
    - `bearer_token_env_var` regex acceptance/rejection.
  - Redaction assertions: error message must not include the offending raw input string(s).
- **Acceptance criteria**:
  - Tests enforce both correctness and safety posture (redaction).
- **Test notes**: keep as pure tests (no subprocess spawning).
- **Risk/rollback notes**: tests-only; safe.

Checklist:
- Implement: table-driven tests for each invalid/valid case.
- Test: `cargo test -p agent_api --features codex,claude_code`.
- Validate: redaction assertions are strict (no substring matches).
- Cleanup: keep test data minimal and readable.

#### S2.T4 — Add “validate-before-hook” gateway test (no spawn / no hook call)

- **Outcome**: A regression test proving invalid requests do not invoke backend hooks when capability is advertised.
- **Inputs/outputs**:
  - Output: a test backend that:
    - advertises the relevant MCP capability id, and
    - increments a counter (or panics) if the hook is invoked.
- **Implementation notes**:
  - Use an invalid request that should deterministically fail validation (e.g., empty/whitespace name).
- **Acceptance criteria**:
  - The hook invocation counter remains 0 and the gateway returns `InvalidRequest`.
- **Test notes**: pure unit test; no filesystem or process usage.
- **Risk/rollback notes**: tests-only; safe.

Checklist:
- Implement: test backend + invalid request case.
- Test: `cargo test -p agent_api --features codex,claude_code`.
- Validate: counter/panic proves “no hook call.”
- Cleanup: keep backend minimal (no unrelated capabilities).

## Notes for downstream seams (non-tasking)

- SEAM-3/4 should call the shared normalization/validation helpers in their hook implementations as a defense-in-depth check (even though the gateway validates), so direct backend usage cannot bypass “validate before spawn.”
