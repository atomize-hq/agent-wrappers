# Threaded Seam Decomposition — SEAM-2 Backend advertising + normalization hook

Pack: `docs/project_management/packs/active/agent-api-model-selection/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-model-selection/seam-2-backend-advertising-normalization.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-model-selection/threading.md`
- Scope brief: `docs/project_management/packs/active/agent-api-model-selection/scope_brief.md`
- Canonical specs:
  - `docs/specs/universal-agent-api/extensions-spec.md`
  - `docs/specs/universal-agent-api/capabilities-schema-spec.md`
  - `docs/specs/universal-agent-api/capability-matrix.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-2
- **Name**: Backend advertising + normalization hook
- **Goal / value**: Give both built-in backends one backend-neutral, trim-first model-selection handoff and keep public advertising truthful, so SEAM-3/4 can map a typed `Option<String>` instead of re-parsing raw extension JSON.
- **Type**: integration
- **Scope**
  - In:
    - add one shared model-selection normalization helper owned by `backend_harness`
    - keep R0 allowlist gating ahead of model parsing and preserve the pinned safe InvalidRequest message
    - carry the normalized `Option<String>` through `NormalizedRequest` so backend mapping seams consume typed output only
    - couple built-in backend `supported_extension_keys()` and `capabilities()` posture for `agent_api.config.model.v1`
    - regenerate `docs/specs/universal-agent-api/capability-matrix.md` in the same change that flips built-in advertising
  - Out:
    - Codex `--model <trimmed-id>` argv wiring and fork runtime rejection behavior
    - Claude Code `--model <trimmed-id>` argv wiring and runtime rejection behavior
    - backend runtime rejection fixtures and end-to-end regression coverage owned by SEAM-5
- **Primary interfaces (contracts)**
  - Produced (owned):
    - `MS-C05 — Built-in advertising contract`
    - `MS-C08 — Capability-matrix publication handoff`
    - `MS-C09 — Shared model-normalizer handoff`
  - Consumed (required upstream):
    - `MS-C01 — Universal model-selection extension key`
    - `MS-C02 — Absence semantics`
    - `MS-C03 — Pre-spawn validation contract`
- **Key invariants / rules**
  - `agent_api.config.model.v1` is parsed in exactly one harness-owned place after R0 allowlist gating.
  - The helper returns `Result<Option<String>, AgentWrapperError>` and never echoes the raw model id in errors.
  - Downstream seams consume typed normalized output and MUST NOT add a second raw parser.
  - Built-in advertising and backend allowlists stay coupled: a backend MUST NOT advertise the key without admitting the same key at R0, and MUST NOT admit the key at R0 while leaving advertising false for the same deterministic-support posture.
  - Capability publication is part of this seam, but the advertising flip must land only in the integration change that already includes the deterministic flow outcomes owned by SEAM-3/4.
- **Touch surface**
  - `crates/agent_api/src/backend_harness/normalize.rs`
  - `crates/agent_api/src/backend_harness/contract.rs`
  - `crates/agent_api/src/backends/codex/backend.rs`
  - `crates/agent_api/src/backends/codex/policy.rs`
  - `crates/agent_api/src/backends/codex/harness.rs`
  - `crates/agent_api/src/backends/claude_code/backend.rs`
  - `crates/agent_api/src/backends/claude_code/mod.rs`
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `docs/specs/universal-agent-api/capability-matrix.md`
- **Verification**
  - harness unit tests prove absent / non-string / empty-after-trim / oversize-after-trim / trimmed-success cases and preserve the exact safe message `invalid agent_api.config.model.v1`
  - backend capability tests prove `supported_extension_keys()` and `capabilities()` do not drift for the model key
  - `cargo run -p xtask -- capability-matrix` updates the generated matrix in the same change as the advertising flip
- **Threading constraints**
  - Upstream blockers: SEAM-1
  - Downstream blocked seams: SEAM-3, SEAM-4, SEAM-5B
  - Contracts produced (owned): `MS-C05`, `MS-C08`, `MS-C09`
  - Contracts consumed: `MS-C01`, `MS-C02`, `MS-C03`

## Slicing Strategy

**Contract-first with a truthful-publication tail slice**: land the shared normalizer and typed handoff first, then wire backend exposure surfaces so R0 admission and public advertising cannot drift, and finalize matrix publication only in the same integration change that already proves the downstream Codex and Claude flow mappings are deterministic.

## Vertical Slices

- **S1 — Shared model normalizer + normalized-request handoff**
  - File: `docs/project_management/packs/active/agent-api-model-selection/threaded-seams/seam-2-backend-advertising-normalization/slice-1-shared-model-normalizer.md`
- **S2 — Backend exposure gates + no-second-parser adoption**
  - File: `docs/project_management/packs/active/agent-api-model-selection/threaded-seams/seam-2-backend-advertising-normalization/slice-2-backend-exposure-gates.md`
- **S3 — Capability publication + conformance gate**
  - File: `docs/project_management/packs/active/agent-api-model-selection/threaded-seams/seam-2-backend-advertising-normalization/slice-3-capability-publication-and-conformance.md`

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `MS-C09 — Shared model-normalizer handoff`: produced by S1 in `crates/agent_api/src/backend_harness/normalize.rs` and `crates/agent_api/src/backend_harness/contract.rs`, yielding one typed `Option<String>` handoff on `NormalizedRequest`.
  - `MS-C05 — Built-in advertising contract`: wired by S2 across `supported_extension_keys()` and `capabilities()` so built-in backends expose `agent_api.config.model.v1` only when the flow set has one deterministic v1 outcome.
  - `MS-C08 — Capability-matrix publication handoff`: completed by S3 via `docs/specs/universal-agent-api/capability-matrix.md` regeneration in the same change as the advertising flip.
- **Contracts consumed**:
  - `MS-C01`: S1 uses the canonical key id and trim-first semantics.
  - `MS-C02`: S1 preserves `Ok(None)` absence behavior so downstream mapping omits `--model`.
  - `MS-C03`: S1 owns the pinned safe InvalidRequest behavior and byte bound.
  - `MS-C06` / `MS-C07`: S3 consumes the already-landed downstream mapping posture as readiness evidence for the final advertising flip; ownership remains with SEAM-3 / SEAM-4.
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-2`: all slices assume the owner-spec semantics and safe InvalidRequest posture are already pinned.
  - `SEAM-2 blocks SEAM-3`: S1 must land before Codex mapping can consume a typed normalized model id instead of raw request parsing.
  - `SEAM-2 blocks SEAM-4`: S1 must land before Claude mapping can consume a typed normalized model id instead of raw request parsing.
  - `SEAM-2 blocks SEAM-5B`: S2/S3 provide the truthful capability posture and published matrix that backend/runtime regression tests must assert.
- **Parallelization notes**:
  - What can proceed now:
    - S1 can start immediately after SEAM-1 verification closes.
    - S2 can be prepared in parallel with late S1 review, as long as it does not introduce a second parser or split allowlist and advertising decisions.
  - What must wait:
    - The final advertising flip and matrix publication in S3 must wait for the integration change that already carries the deterministic mapping outcomes from SEAM-3 / SEAM-4.
    - SEAM-5B should wait for S2/S3 so its assertions target the final capability posture rather than an intermediate state.

## Integration suggestions (explicitly out-of-scope for SEAM-2 tasking)

- SEAM-3 and SEAM-4 should read `NormalizedRequest.model_selection` (or the equivalent typed field) and treat any direct `request.extensions["agent_api.config.model.v1"]` parse as a review failure.
- WS-INT should treat any stale `capability-matrix.md` diff or any new raw parser outside `backend_harness/normalize.rs` as merge-blocking.
