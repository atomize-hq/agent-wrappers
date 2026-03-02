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
      - `agent_kind: AgentWrapperKind` (for error reporting; rendered via `.as_str()`)
      - supported extension keys set
      - backend-specific “extract policy” function (optional; returns typed policy struct)
    - Backend defaults (v1): env defaults + default timeout only (see “Backend config defaults included in normalization” below).
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

## Canonical normalization contract (BH-C02/BH-C03) (normative for this pack)

All of the following are internal-only and live in `crates/agent_api/src/backend_harness.rs`.

### Extension key matching rules (exact)

Extension keys are capability ids (per `docs/specs/universal-agent-api/capabilities-schema-spec.md`):

- Comparison is **exact string match** (case-sensitive).
- No trimming or Unicode normalization is applied.
- Backends MUST NOT accept aliases for keys in v1.
- Duplicate keys are not representable in `AgentWrapperRunRequest.extensions` because it is a
  `BTreeMap<String, serde_json::Value>`; therefore, “duplicate key” behavior is undefined and
  out-of-scope for this pack.

Namespaces:

- Core keys: `agent_api.*` (schema + defaults owned by `docs/specs/universal-agent-api/extensions-spec.md`).
- Backend keys: `backend.<agent_kind>.*` (schema + defaults owned by the backend’s authoritative docs).

### Backend config defaults included in normalization (v1)

Normalization consumes only:

- `BackendDefaults.env`
- `BackendDefaults.default_timeout`

Other backend config defaults (including `working_dir`, binary paths, or backend-specific config)
are **excluded** from SEAM-2 normalization and remain backend-owned.

### Normalized request schema (exact)

The internal normalized request is:

```rust
pub(crate) struct NormalizedRequest<P> {
    pub agent_kind: AgentWrapperKind,
    pub prompt: String,
    pub working_dir: Option<std::path::PathBuf>,
    pub effective_timeout: Option<std::time::Duration>,
    pub env: std::collections::BTreeMap<String, String>,
    pub policy: P,
}
```

Preservation/discard rules (normative):

- Preserved:
  - `prompt`, `working_dir`, `env` (as derived), `effective_timeout` (as derived), and extracted `policy`.
- Discarded:
  - Raw `request.extensions` values are NOT retained in `NormalizedRequest`.
    - They are only read during validation and policy extraction.
    - Error messages MUST NOT dump raw JSON values.

### Effective timeout semantics (exact)

Inputs:
- `request.timeout: Option<Duration>`
- `defaults.default_timeout: Option<Duration>`

Derived:
- `effective_timeout: Option<Duration>` MUST be:
  - `Some(t)` when `request.timeout == Some(t)` (including `t == Duration::ZERO`, which is an explicit “no timeout” request), else
  - `defaults.default_timeout` when `request.timeout == None`.

Explicit “no timeout” (`Duration::ZERO`) semantics (pinned for this pack):
- If `effective_timeout == Some(Duration::ZERO)`, adapters MUST treat it as “disable timeout”
  (MUST NOT interpret it as “timeout immediately”).

### Normalization function signature + call order (exact)

Normalization is a single harness-owned entrypoint:

```rust
pub(crate) fn normalize_request<A: BackendHarnessAdapter>(
    adapter: &A,
    defaults: &BackendDefaults,
    request: AgentWrapperRunRequest,
) -> Result<NormalizedRequest<A::Policy>, AgentWrapperError>;
```

Call order is fixed (normative):

1) Universal invalid-request checks (must be behavior-preserving):
   - `request.prompt.trim().is_empty()` MUST return `AgentWrapperError::InvalidRequest`.
2) `BH-C02` fail-closed allowlist validation:
   - Reject unknown extension keys as `AgentWrapperError::UnsupportedCapability`.
3) Backend-specific policy extraction:
   - `adapter.validate_and_extract_policy(&request)` (known keys only).
4) `BH-C03` env merge:
   - `defaults.env` overridden by `request.env` (request wins on key collision).
5) `BH-C03` timeout derivation:
   - derive `effective_timeout` per the pinned rule above.
6) Construct and return `NormalizedRequest { ... }`.

## Downstream decomposition prompt

Decompose into slices that (1) implement an allowlist-based extension validator, (2) implement env merge + timeout derivation helpers, and (3) add focused unit tests proving deterministic precedence and fail-closed behavior.
