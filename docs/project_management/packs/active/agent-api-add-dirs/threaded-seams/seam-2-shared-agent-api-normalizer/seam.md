# Threaded Seam Decomposition — SEAM-2 Shared `agent_api` add-dir normalizer

Pack: `docs/project_management/packs/active/agent-api-add-dirs/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-add-dirs/seam-2-shared-agent-api-normalizer.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-add-dirs/threading.md`
- Scope brief: `docs/project_management/packs/active/agent-api-add-dirs/scope_brief.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-2
- **Name**: shared add-dir request parsing / validation / resolution helper
- **Goal / value**: compute the effective add-dir set exactly once inside `agent_api` so Codex and Claude Code consume the same validated `Vec<PathBuf>` and the same safe error posture.
- **Type**: integration
- **Scope**
  - In:
    - Add one shared helper entrypoint in `crates/agent_api/src/backend_harness/normalize.rs`:
      `normalize_add_dirs_v1(raw: Option<&serde_json::Value>, effective_working_dir: &Path) -> Result<Vec<PathBuf>, AgentWrapperError>`.
    - Enforce the pinned v1 contract for `agent_api.exec.add_dirs.v1`: closed object schema, `dirs` bounds, trim, relative resolution, lexical normalization, `exists && is_dir`, dedup preserving first occurrence order, and safe `InvalidRequest` templates.
    - Re-export the helper through the backend harness module so backend adapters can call it without opening the raw extension payload themselves.
    - Update Codex and Claude policy extraction so `validate_and_extract_policy(...)` computes the effective working directory, calls the helper once, and stores the normalized `Vec<PathBuf>` on the backend policy before any spawn-specific mapping.
    - Capture Claude run-start cwd in the backend entrypoints so its effective-working-directory ladder matches the pack’s pinned precedence.
  - Out:
    - Capability advertising for `agent_api.exec.add_dirs.v1`.
    - Backend-specific argv emission and session-branch placement.
    - Runtime rejection parity and capability-matrix regeneration.
- **Primary interfaces (contracts)**
  - Produced (owned):
    - `AD-C02 — Effective add-dir set algorithm`
  - Consumed (required upstream):
    - `AD-C01 — Core add-dir extension key`
    - `AD-C03 — Safe error posture`
    - `AD-C07 — Absence semantics`
    - `AD-C04 — Session-flow parity` as a sequencing constraint on where the helper is invoked
- **Key invariants / rules**
  - The shared helper is the only place that decides trim, relative resolution, lexical normalization, filesystem validation, and dedup behavior for this key.
  - The helper returns `Ok(Vec::new())` when the extension key is absent.
  - The helper must not compute backend defaults or its own working-directory precedence ladder; it only consumes the already-selected effective working directory.
  - Backend policy and spawn layers must consume `Vec<PathBuf>` and must not reread `request.extensions["agent_api.exec.add_dirs.v1"]`.
  - Safe `InvalidRequest` messages for this key are limited to `invalid agent_api.exec.add_dirs.v1`, `invalid agent_api.exec.add_dirs.v1.dirs`, and `invalid agent_api.exec.add_dirs.v1.dirs[<i>]`.
- **Touch surface (code)**
  - `crates/agent_api/src/backend_harness/mod.rs`
  - `crates/agent_api/src/backend_harness/normalize.rs`
  - `crates/agent_api/src/backend_harness/normalize/tests.rs`
  - `crates/agent_api/src/backends/codex/policy.rs`
  - `crates/agent_api/src/backends/codex/harness.rs`
  - `crates/agent_api/src/backends/claude_code/backend.rs`
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - backend-local tests under `crates/agent_api/src/backends/codex/tests/` and `crates/agent_api/src/backends/claude_code/tests/`
- **Verification**
  - Helper-level unit coverage for schema, bounds, normalization, filesystem validation, dedup order, and safe message selection.
  - Direct policy-extraction tests for Codex and Claude proving the helper is called with the effective working directory and that the resulting `Vec<PathBuf>` lands on policy objects.
  - No backend argv assertions in this seam; those remain in SEAM-3/4/5.
- **Threading constraints**
  - Upstream blockers: `SEAM-1`
  - Downstream blocked seams: `SEAM-3`, `SEAM-4`, `SEAM-5`
  - Contracts produced (owned): `AD-C02`
  - Contracts consumed: `AD-C01`, `AD-C03`, `AD-C07`, `AD-C04` (sequencing only)

## Slicing Strategy

**Contract-first / dependency-first**: `SEAM-2` is the shared contract publisher for downstream backend work. Land the reusable helper and safe error semantics first, then wire both backend policy layers to consume the helper exactly once, then lock the full edge-case matrix with conformance tests.

## Vertical Slices

- `S1` → `slice-1-shared-helper-contract.md`: publish `normalize_add_dirs_v1(...)` and the reusable normalization/error contract.
- `S2` → `slice-2-backend-policy-handoff.md`: wire Codex and Claude policy extraction to compute effective cwd and carry normalized `Vec<PathBuf>`.
- `S3` → `slice-3-conformance-and-drift-guards.md`: pin the exhaustive helper edge cases and backend handoff regressions.

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `AD-C02`: effective add-dir set algorithm.
    - Definition: backend adapters resolve the effective working directory, call `normalize_add_dirs_v1(...)`, receive `Vec<PathBuf>`, and never reread the raw extension payload.
    - Where it lives: `crates/agent_api/src/backend_harness/normalize.rs` plus backend policy extraction in `crates/agent_api/src/backends/codex/harness.rs` and `crates/agent_api/src/backends/claude_code/harness.rs`.
    - Produced by: `S1` publishes the helper contract; `S2` completes the backend-policy handoff.
- **Contracts consumed**:
  - `AD-C01`: schema + bounds for `agent_api.exec.add_dirs.v1`; consumed by `S1` when parsing the closed object and validating `dirs`.
  - `AD-C03`: safe invalid-message templates; consumed by `S1` for field/index error selection and by `S3` for no-leak regression coverage.
  - `AD-C07`: absence means no synthesized directories and no argv emission; consumed by `S1` via `Ok(Vec::new())` and by `S2` when storing an empty policy list.
  - `AD-C04`: session selectors stay orthogonal and accepted add-dir inputs must survive into session flows; consumed by `S2` by keeping helper invocation inside `validate_and_extract_policy(...)` after selector parsing and before spawn-specific mapping.
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-2`: every slice assumes SEAM-1 has already pinned the schema, absence semantics, and safe error posture.
  - `SEAM-2 blocks SEAM-3`: `S2` gives Codex mapping code a normalized `Vec<PathBuf>` and removes any reason to parse raw extension payloads in SEAM-3.
  - `SEAM-2 blocks SEAM-4`: `S2` gives Claude mapping code the same normalized `Vec<PathBuf>` and the same effective-working-directory handoff.
  - `SEAM-2 blocks SEAM-5`: `S3` publishes the exhaustive regression matrix SEAM-5 can reuse instead of re-specifying helper semantics.
- **Parallelization notes**:
  - What can proceed now: `S1` can land immediately after SEAM-1; `S3` test scaffolding can start once `S1` stabilizes.
  - What must wait: `S2` depends on the helper signature from `S1`; SEAM-3/4 should not implement argv mapping until `S2` lands; SEAM-5 should wait for `S2` and the final `S3` regression matrix.
