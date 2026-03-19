# SEAM-4 — Claude Code backend mapping (threaded decomposition)

> Pack: `docs/project_management/packs/active/agent-api-model-selection/`
> Seam brief: `seam-4-claude-code-mapping.md`
> Threading source of truth: `threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-4
- **Name**: Claude Code backend mapping
- **Goal / value**: make `agent_api.config.model.v1` deterministically drive Claude Code print-mode model selection through the existing request/argv path, while keeping `--fallback-model` out of scope and translating late runtime rejection onto a safe backend-error path.
- **Type**: capability (backend mapping)
- **Slicing strategy**: dependency-first into conformance. Consume SEAM-2's normalized helper output first, then land the print/session argv mapping and explicit `--fallback-model` exclusion, then pin the runtime/error invariants and contract/test surfaces that SEAM-5B depends on.
- **Scope**
  - In:
    - consume the shared `Result<Option<String>, AgentWrapperError>` model-normalizer handoff from `crates/agent_api/src/backend_harness/normalize.rs`
    - plumb `Some(trimmed_model_id)` into Claude Code print/session flows via `ClaudePrintRequest::model(...)`
    - preserve absence semantics by omitting `.model(...)` / `--model`
    - explicitly keep `agent_api.config.model.v1` from mapping to `--fallback-model` or other secondary overrides
    - translate backend-owned runtime model rejection into safe `AgentWrapperError::Backend` outcomes and one terminal `Error` event when the stream is already open
    - update Claude-facing normative docs/tests so the mapping and error posture are reviewable and regression-safe
  - Out:
    - ownership of extension parsing, trimming, schema bounds, or InvalidRequest messaging (SEAM-1 / SEAM-2)
    - capability advertising and capability-matrix publication (SEAM-2)
    - backend-agnostic regression coverage across all seams (SEAM-5)
- **Touch surface**:
  - `crates/agent_api/src/backends/claude_code/backend.rs`
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backends/claude_code/mapping.rs`
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`
  - `crates/agent_api/src/backends/claude_code/tests/mapping.rs`
  - `crates/agent_api/src/backends/claude_code/tests/support.rs`
  - `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`
  - `crates/agent_api/src/backend_harness/runtime.rs`
  - `crates/claude_code/src/commands/print.rs`
  - `crates/claude_code/tests/root_flags_argv.rs`
  - `docs/specs/claude-code-session-mapping-contract.md`
- **Verification**:
  - unit/integration coverage proves fresh print, resume, and fork flows emit exactly one `--model <trimmed-id>` and preserve absence semantics
  - focused argv coverage proves `--model <trimmed-id>` stays before any `--add-dir` group, session-selector flags, `--fallback-model`, and the final `--verbose` token
  - negative coverage proves the universal key never maps to `--fallback-model`
  - runtime rejection coverage proves completion and terminal `Error` event share the same safe message after `system init`, with no raw model id/stdout/stderr leakage
  - spec diff review proves Claude contract docs match the final implementation shape and argv ordering
- **Threading constraints**
  - Upstream blockers: SEAM-1 for pinned semantics; SEAM-2 for MS-C05 advertising posture and MS-C09 shared helper output
  - Downstream blocked seams: SEAM-5B
  - Contracts produced (owned): MS-C07
  - Contracts consumed: MS-C02, MS-C04, MS-C05, MS-C09

Implementation note: treat `threading.md` plus `docs/specs/claude-code-session-mapping-contract.md` as authoritative. This decomposition does not change ownership or directionality; it turns those constraints into conflict-safe implementation slices.

## Slice index

- `S1` → `slice-1-model-handoff.md`: adopt SEAM-2's normalized model helper in Claude policy/harness wiring so the backend never reparses the raw extension key.
- `S2` → `slice-2-print-session-argv-conformance.md`: map fresh/resume/fork print flows through `ClaudePrintRequest::model(...)` and pin the `--fallback-model` exclusion and argv ordering.
- `S3` → `slice-3-runtime-rejection-conformance.md`: harden runtime rejection/error-event translation and update the Claude contract/test surfaces that SEAM-5B will rely on.

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `MS-C07`: Claude mapping contract. Claude Code consumes the effective trimmed model id and emits exactly one `--model <trimmed-id>` through the existing print request / argv path, before any `--add-dir` group, session-selector flags, or `--fallback-model`; this key carries no fallback-model or secondary override semantics.
    - Canonical locations: `threading.md`, `docs/specs/claude-code-session-mapping-contract.md`
    - Produced by: `S1` (typed handoff adoption), `S2` (print/session mapping + fallback exclusion), `S3` (runtime/error conformance + contract publication)
- **Contracts consumed**:
  - `MS-C02`: absence semantics owned by SEAM-1.
    - Consumed by: `S1.T1` and `S2.T1` so missing `agent_api.config.model.v1` never synthesizes `.model(...)` or `--model`.
  - `MS-C04`: backend-owned runtime rejection contract owned by SEAM-1.
    - Consumed by: `S3.T1` and `S3.T2` to translate runtime model rejection into safe backend errors and one terminal `Error` event when applicable.
  - `MS-C05`: built-in advertising contract owned by SEAM-2.
    - Consumed by: `S1`/`S2` as a reachability assumption only; Claude mapping is valid only once every exposed print/session flow is deterministic after SEAM-2 lands.
  - `MS-C09`: shared model-normalizer handoff owned by SEAM-2.
    - Consumed by: `S1.T1` and `S2.T1`; SEAM-4 must consume only the typed `Option<String>` output and must not re-parse raw extensions.
- **Dependency edges honored**:
  - `SEAM-1 gates SEAM-4`: this plan assumes the canonical semantics in `docs/specs/universal-agent-api/extensions-spec.md` are already pinned and only implements Claude-side conformance.
  - `SEAM-2 blocks SEAM-4`: `S1` and `S2` explicitly depend on the shared helper output from `crates/agent_api/src/backend_harness/normalize.rs`; no task in this seam adds a second parser.
  - `SEAM-4 blocks SEAM-5B`: `S1`/`S2`/`S3` together provide the final Claude mapping and safe runtime-error behavior that SEAM-5B must assert.
- **Parallelization notes**:
  - What can proceed now: draft spec/test cases and fake-Claude scenario hooks in `S3`; prep localized Claude harness/module changes once SEAM-2's helper signature is stable.
  - What must wait: landing `S1`/`S2` requires SEAM-2's shared helper; SEAM-5B must wait for all three slices because the tests pin both mapping and backend-error behavior.
