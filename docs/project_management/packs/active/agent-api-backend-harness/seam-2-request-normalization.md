# SEAM-2 — Canonical request normalization + validation

- **Name**: Shared request invariants (validation, env merge, timeout wrapping)
- **Type**: integration
- **Goal / user value**: Ensure every backend applies the same universal invariants (and fails closed) so semantics do not drift across backends.
- **Scope**
  - In:
    - Centralize fail-closed extension key validation against a backend-provided allowlist.
    - Centralize shared parsing helpers for extension values (e.g., `bool`, `string enum`) where appropriate.
    - Centralize env merge precedence rules:
      - backend config env (defaults) overridden by `AgentWrapperRunRequest.env`.
    - Centralize timeout derivation/wrapping rules (request timeout overrides backend default).
    - Centralize shared “invalid request” checks that are universal (e.g., prompt must be non-empty).
      - Evidence (current behavior, both backends): `crates/agent_api/src/backends/codex.rs` and `crates/agent_api/src/backends/claude_code.rs` already reject `request.prompt.trim().is_empty()`.
  - Out:
    - Backend-specific validation logic that is truly backend-specific (e.g., Codex’s sandbox/approval enums, Claude’s `permission_mode` mapping) — those remain in the backend adapter but should plug into the harness hook(s).
    - Any change to the normative extension key set.
- **Primary interfaces (contracts)**
  - Inputs:
    - `AgentWrapperRunRequest`
    - Backend-provided:
      - `agent_kind` string (for error reporting)
      - supported extension keys set
      - backend-specific “extract policy” function (optional; returns typed policy struct)
    - Backend config defaults (env, timeout, working_dir, etc.) as currently modeled.
  - Outputs:
    - A “normalized request” struct (internal) used by the harness to spawn.
    - `AgentWrapperError::{UnsupportedCapability, InvalidRequest, Backend}` with stable, redacted messages.
- **Key invariants / rules**:
  - Unknown extension keys MUST error before spawn as `UnsupportedCapability` (fail closed).
  - Env precedence MUST be deterministic and consistent across backends.
  - Timeout semantics MUST be consistent across backends (including “absent” behavior).
- **Dependencies**
  - Blocks:
    - `SEAM-5` — backend migration should not re-implement these rules.
  - Blocked by:
    - `SEAM-1` — the harness contract defines where normalization happens and what it returns.
- **Touch surface**:
  - `crates/agent_api/src/backend_harness.rs` (or sibling internal module)
  - Existing backend adapters:
    - `crates/agent_api/src/backends/codex.rs`
    - `crates/agent_api/src/backends/claude_code.rs`
- **Verification**:
  - Unit tests at the harness layer for:
    - fail-closed unknown extension key behavior
    - env merge precedence
    - timeout derivation (request vs backend defaults)
- **Risks / unknowns**
  - Risk: “normalization” subtly changes backend-specific behavior (e.g., a backend’s current default differs).
  - De-risk plan: for each normalized field, pin a comparison test against current backend behavior before/after migration (SEAM-5).
- **Rollout / safety**:
  - Ship behind refactor-only changes; rely on existing backend tests + new harness tests.

## Downstream decomposition prompt

Decompose into slices that (1) implement an allowlist-based extension validator, (2) implement env merge + timeout derivation helpers, and (3) add focused unit tests proving deterministic precedence and fail-closed behavior.
