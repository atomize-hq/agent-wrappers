# Threading — Universal model selection (`agent_api.config.model.v1`)

This section makes coupling explicit: contracts/interfaces, dependency edges, and sequencing.

## Contract registry

- **MS-C01 — Universal model-selection extension key**
  - **Type**: config (core extension key)
  - **Definition**: `agent_api.config.model.v1` is a string-valued extension key whose effective value is the
    caller-supplied model id after trimming leading/trailing Unicode whitespace.
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-2/3/4/5

- **MS-C02 — Absence semantics**
  - **Type**: policy
  - **Definition**: when `agent_api.config.model.v1` is absent, the backend MUST NOT emit `--model`, MUST NOT infer a
    model id, and MUST preserve its existing default model-selection behavior.
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-3/4/5

- **MS-C03 — Pre-spawn validation contract**
  - **Type**: schema
  - **Definition**: after R0 capability gating, the key MUST validate before spawn as:
    - JSON string only,
    - trimmed value non-empty,
    - trimmed value length `<= 128` UTF-8 bytes.
    Failures resolve as `AgentWrapperError::InvalidRequest`.
  - **Pinned InvalidRequest message**: pre-spawn failures for this key MUST use the exact safe template
    `invalid agent_api.config.model.v1` and MUST NOT echo the raw model id.
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-2/3/4/5

- **MS-C04 — Backend-owned runtime rejection contract**
  - **Type**: integration
  - **Definition**: if the key passed R0 capability gating and pre-spawn validation (MS-C03), but the backend later
    determines that the requested model id cannot be honored at runtime (unknown, unavailable, unauthorized, or the
    targeted run flow cannot apply an accepted model id), the run MUST resolve as
    `AgentWrapperError::Backend { message }`, where `message` is safe/redacted and does not embed raw stdout/stderr. If
    the stream is already open, exactly one terminal `AgentWrapperEventKind::Error` event with the same safe message
    is emitted before closure.
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-3/4/5

- **MS-C05 — Built-in advertising contract**
  - **Type**: permission
  - **Definition**: built-in backends advertise `agent_api.config.model.v1` only when every run flow they expose has a
    deterministic v1 outcome after R0 gating and pre-spawn validation: either the flow applies the accepted effective
    trimmed model id unchanged to its underlying transport/CLI, or it takes a pinned backend-owned safe rejection path.
    A flow that silently drops, rewrites, or conditionally ignores an accepted model id is not deterministic support.
    Because `AgentWrapperCapabilities.ids` is backend-global rather than per-flow, built-in advertising can remain
    unconditional only when every exposed flow meets one of those two outcomes. For v1 this means Codex may advertise
    globally once exec/resume map to `--model <trimmed-id>` and fork preserves the pinned pre-handle safe rejection
    path from `docs/specs/codex-app-server-jsonrpc-contract.md`; Claude Code may advertise globally once print
    exec/resume/fork all emit exactly one `--model <trimmed-id>` pair per
    `docs/specs/claude-code-session-mapping-contract.md`.
  - **Owner seam**: SEAM-2
  - **Consumers**: SEAM-3/4/5

- **MS-C09 — Shared model-normalizer handoff**
  - **Type**: integration
  - **Definition**: SEAM-2 owns one shared helper in `crates/agent_api/src/backend_harness/normalize.rs` that reads
    `request.extensions["agent_api.config.model.v1"]` only after R0 gating and exports
    `Result<Option<String>, AgentWrapperError>`, where `None` means absent, `Some(trimmed_model_id)` means valid, and
    `InvalidRequest { message: "invalid agent_api.config.model.v1" }` covers every invalid input shape/bounds case.
    SEAM-3 and SEAM-4 consume only that typed output and MUST NOT parse the raw extension payload again.
  - **Owner seam**: SEAM-2
  - **Consumers**: SEAM-3/4/5

- **MS-C08 — Capability-matrix publication handoff**
  - **Type**: release/integration
  - **Definition**: SEAM-2 owns regenerating `docs/specs/universal-agent-api/capability-matrix.md` in the same
    change that updates built-in advertising for `agent_api.config.model.v1`. SEAM-5 consumes that artifact for
    regression assertions, and WS-INT reruns `cargo run -p xtask -- capability-matrix`; any stale matrix diff blocks
    merge.
  - **Owner seam**: SEAM-2
  - **Consumers**: SEAM-5, WS-INT

- **MS-C06 — Codex mapping contract**
  - **Type**: integration
  - **Definition**: Codex exec/resume mapping consumes the effective trimmed model id and emits exactly one
    `--model <trimmed-id>` through the existing Codex builder/argv path. Codex fork currently has no app-server model
    transport field, so accepted model-selection inputs on fork flows take the pinned pre-handle backend rejection path
    from `docs/specs/codex-app-server-jsonrpc-contract.md`. This key MUST NOT authorize any additional Universal Agent
    API behavior beyond model selection itself.
  - **Owner seam**: SEAM-3
  - **Consumers**: SEAM-5

- **MS-C07 — Claude mapping contract**
  - **Type**: integration
  - **Definition**: Claude Code mapping consumes the effective trimmed model id and emits exactly one
    `--model <trimmed-id>` through the print request / argv path, before any `--add-dir` group, session-selector
    flags, or `--fallback-model`. This key MUST NOT map to `--fallback-model` or any other secondary print-mode
    override unless a separate explicit key exists.
  - **Owner seam**: SEAM-4
  - **Consumers**: SEAM-5

## Dependency graph (text)

- `SEAM-1 gates SEAM-2/3/4` because: backend work starts after the SEAM-1 verification pass confirms there is no unresolved canonical-doc delta. The normative schema itself is already pinned in `docs/specs/universal-agent-api/extensions-spec.md`.
- `SEAM-2 blocks SEAM-3` because: Codex run wiring must consume the shared `Result<Option<String>, AgentWrapperError>` output, not ad hoc raw extension parsing.
- `SEAM-2 blocks SEAM-4` because: Claude run wiring must consume the shared `Result<Option<String>, AgentWrapperError>` output, not ad hoc raw extension parsing.
- `SEAM-1 blocks SEAM-5A` because: pre-mapping validation tests need the pinned InvalidRequest and runtime-rejection posture.
- `SEAM-2 blocks SEAM-5B` because: backend/matrix tests must verify the shared normalization helper, the no-second-parser rule, and capability publication handoff.
- `SEAM-3 blocks SEAM-5B` because: Codex tests must verify the final mapping and backend-error translation behavior.
- `SEAM-4 blocks SEAM-5B` because: Claude tests must verify the final mapping and exclusion of `--fallback-model`.

## Critical path

Implementation critical path:
`SEAM-1 (verification/sync)` → `SEAM-2 (advertising + normalization + matrix publication)` → `SEAM-3/SEAM-4 (backend mapping)` → `SEAM-5B (backend/runtime tests)`

Parallel test path (may proceed after SEAM-1):
`SEAM-1 (verification/sync)` → `SEAM-5A (R0 + schema validation harness tests)`

## Integration points

- **Run extension gate**: `backend_harness::normalize_request()` MUST fail closed on unsupported keys before the shared
  model helper inspects `agent_api.config.model.v1`.
- **Shared helper anchor**: SEAM-2 owns the only raw-extension parse site in
  `crates/agent_api/src/backend_harness/normalize.rs`; downstream backend policy layers consume only the normalized
  `Option<String>` result.
- **Wrapper crate parity**: `codex::CodexClientBuilder` and `claude_code::ClaudePrintRequest` already expose
  `.model(...)`; SEAM-3/4 reuse those APIs and inherit the canonical argv-order rules instead of emitting `--model`
  manually.
- **Single-parser enforcement**: SEAM-2/3/4 verification is incomplete until shared-helper unit tests, backend argv
  tests, and diff review all confirm there is no new direct parse of `agent_api.config.model.v1` outside
  `crates/agent_api/src/backend_harness/normalize.rs`.

## Parallelization notes / conflict-safe workstreams

- **WS-SPEC**: SEAM-1 docs-first contract alignment under `docs/specs/universal-agent-api/`.
- **WS-NORMALIZE**: SEAM-2 capability advertising plus the shared normalization helper in
  `crates/agent_api/src/backend_harness/normalize.rs`, with backend adapters consuming that helper.
- **WS-CODEX**: SEAM-3 Codex request mapping and runtime error translation.
- **WS-CLAUDE**: SEAM-4 Claude Code request mapping and runtime error translation.
- **WS-TESTS**:
  - SEAM-5A covers R0 + schema-validation harness tests and may start once SEAM-1 verification is complete.
  - SEAM-5B covers backend mapping, runtime rejection, and capability-matrix assertions after SEAM-2/3/4 land.
- **WS-INT (Integration)**: rerun `cargo run -p xtask -- capability-matrix`, `make test`, and `make preflight`;
  treat any stale capability-matrix diff as blocking, validate advertised capabilities, and confirm no raw stderr
  leakage in backend failures.

## Pinned decisions / resolved threads

- **Opaque id posture**: v1 standardizes the request surface, not a shared cross-backend model catalog. See MS-C01/MS-C03.
- **Absence means backend default**: missing key never synthesizes a model override. See MS-C02.
- **No secondary routing by implication**: this key cannot imply fallback-model, reasoning tuning, or policy changes. See MS-C06/MS-C07.
- **Runtime unknown-model handling stays backend-owned**: safe `Backend` error translation is required, but a universal rejection string is not. See MS-C04.
- **InvalidRequest message contract**: pre-spawn validation failures use the single safe template
  `invalid agent_api.config.model.v1`; raw model ids must not appear in that message. See MS-C03.
