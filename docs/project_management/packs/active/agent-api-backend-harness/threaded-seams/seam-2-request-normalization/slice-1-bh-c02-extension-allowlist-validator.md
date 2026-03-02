# S1 — Implement `BH-C02` fail-closed extension allowlist validator

- **User/system value**: Prevents semantic drift by rejecting unknown extension keys universally (pre-spawn, fail closed) using a backend-provided allowlist, with stable `UnsupportedCapability(agent_kind, key)` errors.
- **Scope (in/out)**:
  - In:
    - Implement `BH-C02 extension key allowlist + fail-closed validator` as a harness-local, reusable function that runs before spawn.
    - Add small shared parsing helpers for extension values where appropriate (e.g., `bool`, string enums), returning stable `InvalidRequest` errors.
    - Ensure error messages remain stable and redacted (no backend raw output leakage).
  - Out:
    - Any change to the normative extension key set (keys remain backend-provided).
    - Backend-specific policy extraction/mapping (Codex/Claude adapters remain responsible for interpreting their own extension payloads).
    - Streaming pump, drain-on-drop, or completion gating behavior (SEAM-3/SEAM-4).
- **Acceptance criteria**:
  - Unknown extension keys in `AgentWrapperRunRequest.extensions` are rejected pre-spawn as `AgentWrapperError::UnsupportedCapability(agent_kind, key)` (fail closed).
  - The allowlist comes from the SEAM-1 adapter contract surface (no per-backend re-implementation in SEAM-2).
  - Backends can still run backend-specific validation/policy extraction after the allowlist check, via the SEAM-1 hook surface.
  - A harness-owned “normalized request” shape exists (internal), carrying only validated/derived fields onward to spawn.
- **Dependencies**:
  - Contract from SEAM-1: `BH-C01 backend harness adapter interface` (must expose `agent_kind` + supported extension keys surface + backend-specific validation hook surface).
- **Verification**:
  - `cargo test -p agent_api --features codex,claude_code` (or narrower feature combos as appropriate)
  - Focused unit tests proving fail-closed unknown key behavior and stable error shape (see Slice S3).

## Atomic Tasks

#### S1.T1 — Define the `BH-C02` allowlist validator (pre-spawn)

- **Outcome**: A single harness-owned function that validates request extension keys against the backend’s allowlist and returns `UnsupportedCapability` deterministically on the first unknown key.
- **Inputs/outputs**:
  - Input: `AgentWrapperRunRequest.extensions` (and backend `agent_kind` + allowlist from `BH-C01`)
  - Output: `crates/agent_api/src/backend_harness.rs` (new or updated internal module)
- **Implementation notes**:
  - Treat the allowlist as authoritative for the backend, but enforce “unknown key” failure universally (fail closed).
  - Deterministic “first unknown key” selection (normative):
    - Iterate keys in `AgentWrapperRunRequest.extensions.keys()`.
    - Because `extensions` is a `BTreeMap`, iteration order is ascending `String` order (Rust `Ord`).
    - The “first unknown key” is therefore the lexicographically-smallest unknown key.
    - Validation MUST stop at the first unknown key (do not aggregate unknown keys in v1).
  - Allowlist comparison is exact string match:
    - case-sensitive
    - no trimming / Unicode normalization
    - no key aliases
  - Keep error formatting stable; include `agent_kind` + `key`, but not raw extension values.
- **Acceptance criteria**:
  - Validation happens before any backend process spawn is attempted.
  - Error is `UnsupportedCapability(agent_kind, key)` (per `BH-C02` definition in `threading.md`).
- **Test notes**: unit-tested in Slice S3 (and can include a minimal test co-located with the validator).
- **Risk/rollback notes**: internal-only; if ordering issues are discovered, adjust ordering determinism without changing public API.

Checklist:
- Implement: `validate_extension_keys_fail_closed(...)` in the harness.
- Test: unknown key yields `UnsupportedCapability` before spawn.
- Test: if multiple unknown keys are present, the reported key is the lexicographically-smallest one.
- Validate: `make clippy` (warnings are errors).
- Cleanup: keep policy local and auditable (no per-backend copies).

#### S1.T2 — Wire allowlist validation into the harness normalization lifecycle

- **Outcome**: The harness calls `BH-C02` as part of its request normalization path, and only passes validated requests to adapter spawn.
- **Inputs/outputs**:
  - Output: harness run entrypoint in `crates/agent_api/src/backend_harness.rs`
  - Output: normalized request struct used by downstream harness steps (internal)
- **Implementation notes**:
  - Call order should be explicit:
    1) universal invalid request checks (e.g., empty prompt),
    2) `BH-C02` unknown-key allowlist check,
    3) backend-specific validation hook (optional),
    4) env/timeout derivation (S2),
    5) spawn.
  - Traceability note: “prompt must not be empty” is already enforced in both built-in backends today (`crates/agent_api/src/backends/codex.rs`, `crates/agent_api/src/backends/claude_code.rs`). Centralizing it here is intended to preserve behavior, not introduce a new rule.
  - Keep the backend-specific validation hook separate from the unknown-key check to preserve the ownership split.
- **Acceptance criteria**:
  - Unknown key rejection occurs even if the backend-specific hook is absent or permissive.
  - No backend adapter code changes are required for SEAM-2 (adoption happens in SEAM-5).
- **Test notes**: exercised indirectly by Slice S3 tests that call normalization.
- **Risk/rollback notes**: ensure errors are returned without spawning backend processes (no partial side effects).

Checklist:
- Implement: normalization call sequence in the harness.
- Test: normalization rejects unknown keys pre-spawn.
- Validate: compile under `--features codex`, `--features claude_code`, and combined.
- Cleanup: keep normalization helpers private to the harness module.

#### S1.T3 — Add shared extension parsing helpers (minimal, reusable)

- **Outcome**: A small set of helpers for parsing common extension shapes with stable `InvalidRequest` errors.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backend_harness.rs` (or a sibling internal `backend_harness/normalize.rs` module)
- **Implementation notes**:
  - Only centralize parsing that is genuinely shared across backends (e.g., `bool`, small string-enum).
  - Do not “own” backend-specific option sets (leave those as backend-provided enums or mapping functions).
  - Return errors in a redacted/stable form (no raw JSON dumps).
- **Acceptance criteria**:
  - Backends can use helpers when extracting their policy structs, without changing universal semantics.
  - Helpers are covered by at least one unit test (Slice S3).
- **Test notes**: include one success and one failure case per helper.
- **Risk/rollback notes**: keep the helper surface small to avoid becoming a generic parsing framework.

Checklist:
- Implement: parsing helpers (e.g., `parse_bool_ext`, `parse_string_enum_ext`) with stable errors.
- Test: minimal helper tests (happy + unhappy paths).
- Validate: clippy-clean and no new public API.
- Cleanup: document “shared vs backend-specific” boundary in the harness module docs.
