# Threaded Seam Decomposition — SEAM-2 Request normalization + validation

Pack: `docs/project_management/packs/active/agent-api-backend-harness/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-backend-harness/seam-2-request-normalization.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-backend-harness/threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-2
- **Name**: Shared request invariants (validation, env merge, timeout wrapping)
- **Goal / value**: Ensure every backend applies the same universal request invariants (and fails closed) so semantics do not drift across backends.
- **Type**: integration
- **Scope**
  - In:
    - Centralize fail-closed extension key validation against a backend-provided allowlist.
    - Centralize shared parsing helpers for extension values where appropriate.
    - Centralize env merge precedence rules (backend config env overridden by request env).
    - Centralize timeout derivation/wrapping rules (request timeout overrides backend default).
    - Centralize shared “invalid request” checks that are universal (e.g., prompt must be non-empty).
  - Out:
    - Backend-specific validation logic that is truly backend-specific (e.g., Codex sandbox/approval enums, Claude `permission_mode` mapping).
    - Any change to the normative extension key set.
- **Primary interfaces (contracts)**
  - Produced (owned):
    - `BH-C02 extension key allowlist + fail-closed validator`
    - `BH-C03 env merge + timeout derivation`
  - Consumed (required upstream):
    - `BH-C01 backend harness adapter interface` (SEAM-1)
- **Key invariants / rules**
  - Unknown extension keys MUST error before spawn as `UnsupportedCapability` (fail closed).
  - Env precedence MUST be deterministic and consistent across backends.
  - Timeout semantics MUST be consistent across backends (including “absent” behavior).
- **Touch surface (code)**
  - `crates/agent_api/src/backend_harness.rs` (or sibling internal module)
  - Harness consumers (later, SEAM-5): `crates/agent_api/src/backends/codex.rs`, `crates/agent_api/src/backends/claude_code.rs`
- **Verification**
  - Harness-layer unit tests for:
    - fail-closed unknown extension key behavior
    - env merge precedence
    - timeout derivation (request vs backend defaults)
    - universal invalid request checks (e.g., empty prompt)

## Slicing Strategy

**Dependency-first / contract-first**: SEAM-2 blocks SEAM-5 and owns the invariant contracts that prevent drift. Ship `BH-C02` and `BH-C03` as small, harness-local utilities that plug into the SEAM-1 contract lifecycle and are covered by focused unit tests.

## Vertical Slices

- **S1 — Implement `BH-C02` fail-closed extension allowlist validator**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-1-bh-c02-extension-allowlist-validator.md`
- **S2 — Implement `BH-C03` env merge precedence + timeout derivation helpers**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-2-bh-c03-env-timeout-normalization.md`
- **S3 — Harness-layer unit tests for request normalization invariants**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-2-request-normalization/slice-3-normalization-unit-tests.md`

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `BH-C02 extension key allowlist + fail-closed validator`: implemented in `crates/agent_api/src/backend_harness.rs` as a pre-spawn validator (produced by Slice S1).
  - `BH-C03 env merge + timeout derivation`: implemented in `crates/agent_api/src/backend_harness.rs` as deterministic env merge + timeout selection helpers (produced by Slice S2).
- **Contracts consumed**:
  - `BH-C01 backend harness adapter interface` (SEAM-1): provides `agent_kind` and the supported-extension-keys allowlist surface consumed by S1 and used to build normalized spawn inputs.
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-2`: all slices assume SEAM-1 has pinned where normalization runs and how the adapter surfaces supported extension keys.
  - `SEAM-2 blocks SEAM-5`: S1/S2 deliver reusable invariant enforcement so migrated backends do not re-implement extension/env/timeout logic.
- **Parallelization notes**:
  - What can proceed now: planning + implementation in WS-A after SEAM-1 lands.
  - What must wait: SEAM-5 backend adoption should wait for S1/S2 (and ideally S3) so backends reuse harness invariants by construction.

## Integration suggestions (explicitly out-of-scope for SEAM-2 tasking)

- During SEAM-5 migration, delete per-backend copies of extension/env/timeout logic and route through the harness helpers to avoid drift.
- If a backend currently accepts unknown extension keys, treat that as a bug-fix aligned to ADR-0013 (fail closed), not as a “compat” exception.
