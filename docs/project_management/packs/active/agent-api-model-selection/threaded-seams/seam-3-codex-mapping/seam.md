# SEAM-3 — Codex backend mapping (threaded decomposition)

> Pack: `docs/project_management/packs/active/agent-api-model-selection/`
> Seam brief: `seam-3-codex-mapping.md`
> Threading source of truth: `threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-3
- **Name**: Codex backend mapping
- **Goal / value**: make `agent_api.config.model.v1` deterministically drive Codex model selection for exec/resume flows through the existing builder path, while keeping fork flows and runtime model rejection on pinned safe backend-error paths.
- **Type**: capability (backend mapping)
- **Slicing strategy**: dependency-first into conformance. Consume SEAM-2's normalized helper output first, then land the fork-specific rejection posture, then pin the runtime/error invariants and spec drift guards that SEAM-5 depends on.
- **Scope**
  - In:
    - consume the shared `Result<Option<String>, AgentWrapperError>` model-normalizer handoff from `crates/agent_api/src/backend_harness/normalize.rs`
    - plumb `Some(trimmed_model_id)` into Codex exec/resume via `CodexClientBuilder::model(...)`
    - preserve absence semantics by omitting `.model(...)` / `--model`
    - preserve the pinned pre-handle backend rejection path for fork flows that cannot apply model selection
    - translate backend-owned runtime model rejection into safe `AgentWrapperError::Backend` outcomes and one terminal `Error` event when the stream is already open
    - update Codex-facing normative docs/tests so the mapping and error posture are reviewable and regression-safe
  - Out:
    - ownership of extension parsing, trimming, schema bounds, or InvalidRequest messaging (SEAM-1 / SEAM-2)
    - capability advertising and capability-matrix publication (SEAM-2)
    - backend-agnostic regression coverage across all seams (SEAM-5)
- **Touch surface**:
  - `crates/agent_api/src/backends/codex/policy.rs`
  - `crates/agent_api/src/backends/codex/backend.rs`
  - `crates/agent_api/src/backends/codex/exec.rs`
  - `crates/agent_api/src/backends/codex/fork.rs`
  - `crates/agent_api/src/backends/codex/harness.rs`
  - `crates/agent_api/src/backends/codex/tests/mapping.rs`
  - `crates/agent_api/src/backends/codex/tests/app_server.rs`
  - `crates/agent_api/src/backends/codex/tests/backend_contract.rs`
  - `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`
  - `crates/codex/src/builder/mod.rs`
  - `docs/specs/codex-streaming-exec-contract.md`
  - `docs/specs/codex-app-server-jsonrpc-contract.md`
- **Verification**:
  - unit/integration coverage proves exec/resume emit exactly one `--model <trimmed-id>` and preserve absence semantics
  - fork coverage proves accepted model-selection inputs fail before any app-server request with the pinned safe backend message
  - runtime rejection coverage proves completion and terminal `Error` event share the same safe message, with no raw model id/stdout/stderr leakage
  - spec diff review proves Codex contracts match the final implementation shape and argv ordering
- **Threading constraints**
  - Upstream blockers: SEAM-1 for pinned semantics; SEAM-2 for MS-C05 advertising posture and MS-C09 shared helper output
  - Downstream blocked seams: SEAM-5B
  - Contracts produced (owned): MS-C06
  - Contracts consumed: MS-C02, MS-C04, MS-C05, MS-C09

Implementation note: treat `threading.md` plus the Codex contract docs as authoritative. This decomposition does not change ownership or directionality; it turns those constraints into conflict-safe implementation slices.

## Slice index

- `S1` → `slice-1-exec-resume-model-handoff.md`: adopt SEAM-2's normalized model helper in Codex policy/harness wiring and map exec/resume flows through the existing builder argv path.
- `S2` → `slice-2-fork-model-rejection.md`: pin the no-transport fork rejection behavior so accepted model-selection inputs fail safely before any app-server request.
- `S3` → `slice-3-runtime-rejection-conformance.md`: harden runtime rejection/error-event translation and update the Codex contract/test surfaces that SEAM-5 will rely on.

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `MS-C06`: Codex mapping contract. Exec/resume consume the effective trimmed model id and emit exactly one `--model <trimmed-id>` through the existing builder/argv path; fork flows reject accepted model-selection inputs before any app-server request with the pinned safe backend message; the key carries no extra semantics beyond model selection itself.
    - Canonical locations: `threading.md`, `docs/specs/codex-streaming-exec-contract.md`, `docs/specs/codex-app-server-jsonrpc-contract.md`
    - Produced by: `S1` (exec/resume mapping), `S2` (fork rejection path), `S3` (runtime/error conformance + contract publication)
- **Contracts consumed**:
  - `MS-C02`: absence semantics owned by SEAM-1.
    - Consumed by: `S1.T1` and `S1.T2` so missing `agent_api.config.model.v1` never synthesizes `.model(...)` or `--model`.
  - `MS-C04`: backend-owned runtime rejection contract owned by SEAM-1.
    - Consumed by: `S3.T1` and `S3.T2` to translate runtime model rejection into safe backend errors and one terminal `Error` event when applicable.
  - `MS-C05`: built-in advertising contract owned by SEAM-2.
    - Consumed by: `S1`/`S2` as a reachability assumption only; Codex mapping is valid only once every exposed flow is deterministic after SEAM-2 lands.
  - `MS-C09`: shared model-normalizer handoff owned by SEAM-2.
    - Consumed by: `S1.T1` and `S2.T1`; SEAM-3 must consume only the typed `Option<String>` output and must not re-parse raw extensions.
- **Dependency edges honored**:
  - `SEAM-1 gates SEAM-3`: this plan assumes the canonical semantics in `docs/specs/universal-agent-api/extensions-spec.md` are already pinned and only implements Codex-side conformance.
  - `SEAM-2 blocks SEAM-3`: `S1` and `S2` explicitly depend on the shared helper output from `crates/agent_api/src/backend_harness/normalize.rs`; no task in this seam adds a second parser.
  - `SEAM-3 blocks SEAM-5B`: `S1`/`S2`/`S3` together provide the final Codex mapping and safe runtime-error behavior that SEAM-5B must assert.
- **Parallelization notes**:
  - What can proceed now: draft spec/test cases and stage doc updates in `S3`; prep localized Codex module changes once SEAM-2's helper signature is stable.
  - What must wait: landing `S1`/`S2` requires SEAM-2's shared helper; SEAM-5B must wait for all three slices because the tests pin both mapping and backend-error behavior.
