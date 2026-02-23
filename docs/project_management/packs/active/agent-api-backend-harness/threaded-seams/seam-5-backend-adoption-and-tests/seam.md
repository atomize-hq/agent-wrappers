# Threaded Seam Decomposition — SEAM-5 Backend adoption (Codex + Claude) + conformance tests

Pack: `docs/project_management/packs/active/agent-api-backend-harness/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-backend-harness/seam-5-backend-adoption-and-tests.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-backend-harness/threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-5
- **Name**: Codex + Claude backend migration to harness, with harness-level test coverage
- **Goal / value**: Prove the harness is viable and reduces duplication by migrating the two existing built-in backends while preserving behavior, and by keeping shared invariants guarded by harness-owned tests.
- **Type**: capability
- **Scope**
  - In:
    - Refactor:
      - `crates/agent_api/src/backends/codex.rs`
      - `crates/agent_api/src/backends/claude_code.rs`
      to delegate glue/invariants to the harness.
    - Keep backend-specific mapping/adapter logic in backend-owned modules (e.g. Codex’s `backends/codex/mapping.rs` and Claude’s stream-json mapping helpers).
    - Add/adjust test coverage so migration does not reduce confidence (including moving any “backend-local harness invariant tests” to the harness layer where appropriate).
  - Out:
    - Changing capability IDs or extension keys.
    - Large-scale reorganization of wrapper crates.
- **Primary interfaces (contracts)**
  - Produced (owned): none (SEAM-5 is an adoption seam).
  - Consumed (required upstream):
    - `BH-C01 backend harness adapter interface` (SEAM-1)
    - `BH-C02 extension key allowlist + fail-closed validator` (SEAM-2)
    - `BH-C03 env merge + timeout derivation` (SEAM-2)
    - `BH-C04 stream forwarding + drain-on-drop` (SEAM-3)
    - `BH-C05 completion gating integration` (SEAM-4)
- **Key invariants / rules**
  - “No behavior change” intent relative to ADR-0013’s user contract: refactor-only.
  - Codex/Claude MUST NOT re-implement shared invariants post-migration (validation/env/timeout/bounds/pump/gating live in the harness).
  - Every forwarded event MUST pass through bounds enforcement and redaction rules (by construction via the harness).
- **Touch surface (code)**
  - `crates/agent_api/src/backends/codex.rs`
  - `crates/agent_api/src/backends/claude_code.rs`
  - `crates/agent_api/src/backend_harness.rs`
  - Backend tests in:
    - `crates/agent_api/src/backends/codex/tests.rs`
    - `crates/agent_api/src/backends/claude_code/tests.rs`
  - Harness tests (owned by upstream seams; SEAM-5 may add wiring-only coverage if missing).
- **Verification**
  - `cargo test -p agent_api --features codex`
  - `cargo test -p agent_api --features claude_code`
  - `cargo test -p agent_api --features codex,claude_code`

## Slicing Strategy

**Risk-first / dependency-first within WS-B**: migrate one backend (Codex) onto the upstream harness contracts, then add adoption-focused regression coverage (without duplicating harness-owned invariant tests from SEAM-2/3/4), then migrate the second backend (Claude) once the harness wiring is proven stable.

## Vertical Slices

- **S1 — Migrate Codex backend to the harness (behavior-equivalent)**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-5-backend-adoption-and-tests/slice-1-codex-backend-migration.md`
- **S2 — Adoption conformance: update backend tests + add wiring guards**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-5-backend-adoption-and-tests/slice-2-adoption-conformance-tests.md`
- **S3 — Migrate Claude backend to the harness (behavior-equivalent)**
  - File: `docs/project_management/packs/active/agent-api-backend-harness/threaded-seams/seam-5-backend-adoption-and-tests/slice-3-claude-backend-migration.md`

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - None. SEAM-5 consumes `BH-C01`..`BH-C05` and proves them via real backend adoption.
- **Contracts consumed**:
  - `BH-C01 backend harness adapter interface` (SEAM-1): both Codex and Claude must implement the adapter shape and call the harness entrypoint.
  - `BH-C02 extension key allowlist + fail-closed validator` (SEAM-2): backend modules provide allowlists; the harness rejects unknown keys pre-spawn.
  - `BH-C03 env merge + timeout derivation` (SEAM-2): env and timeout are derived once in the harness (request overrides defaults; absence preserved explicitly).
  - `BH-C04 stream forwarding + drain-on-drop` (SEAM-3): backend modules must not ship their own drain loops; they route through the shared pump.
  - `BH-C05 completion gating integration` (SEAM-4): backend modules must not custom-gate completion; they use the canonical gate builder.
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-5`: both migrations require the harness contract and entrypoints to exist.
  - `SEAM-2/3/4 block SEAM-5`: adoption must use the canonical validator/pump/gating path, not re-implement invariants per backend.
- **Parallelization notes**:
  - Safe to do in WS-B after WS-A merges: Codex and Claude migrations touch primarily `crates/agent_api/src/backends/*` and their mapping helpers.
  - Avoid conflicts: do not modify `crates/agent_api/src/backend_harness.rs` behavior here; treat any needed harness changes as upstream seam follow-ups (or WS-INT reconciliation).

## Integration suggestions (explicitly out-of-scope for SEAM-5 tasking)

- After both migrations land, treat “new backend must use the harness” as a convention gate (code review + lightweight conformance checks).
- If adoption reveals mismatched semantics between Codex and Claude (timeouts, error mapping, ordering), pin the correct harness behavior in the harness-layer tests owned by SEAM-2/3/4 rather than adding per-backend exceptions.
