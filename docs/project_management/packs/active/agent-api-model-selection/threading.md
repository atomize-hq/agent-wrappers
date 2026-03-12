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
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-2/3/4/5

- **MS-C04 — Backend-owned runtime rejection contract**
  - **Type**: integration
  - **Definition**: if a syntactically valid, supported model id is later rejected by the backend runtime as unknown,
    unavailable, or unauthorized, the run MUST resolve as `AgentWrapperError::Backend { message }`, where `message` is
    safe/redacted and does not embed raw stdout/stderr. If the stream is already open, exactly one terminal
    `AgentWrapperEventKind::Error` event with the same safe message is emitted before closure.
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-3/4/5

- **MS-C05 — Built-in advertising contract**
  - **Type**: permission
  - **Definition**: built-in Codex and Claude Code backends advertise `agent_api.config.model.v1` exactly when the
    implementation can deterministically normalize the value and map it to the underlying CLI `--model <id>` surface.
    For v1, that is expected to be unconditional once the implementation lands.
  - **Owner seam**: SEAM-2
  - **Consumers**: SEAM-3/4/5

- **MS-C06 — Codex mapping contract**
  - **Type**: integration
  - **Definition**: Codex mapping consumes the effective trimmed model id and emits exactly one `--model <trimmed-id>`
    through the existing Codex builder/argv path. This key MUST NOT authorize any additional Universal Agent API
    behavior beyond model selection itself.
  - **Owner seam**: SEAM-3
  - **Consumers**: SEAM-5

- **MS-C07 — Claude mapping contract**
  - **Type**: integration
  - **Definition**: Claude Code mapping consumes the effective trimmed model id and emits exactly one
    `--model <trimmed-id>` through the print request / argv path. This key MUST NOT map to `--fallback-model` or any
    other secondary print-mode override unless a separate explicit key exists.
  - **Owner seam**: SEAM-4
  - **Consumers**: SEAM-5

## Dependency graph (text)

- `SEAM-1 blocks SEAM-2` because: backend advertising and normalization need the final schema, trimming, and absence semantics.
- `SEAM-1 blocks SEAM-3` because: Codex mapping needs the pinned distinction between pre-spawn validation and backend-owned runtime rejection.
- `SEAM-1 blocks SEAM-4` because: Claude mapping needs the pinned distinction between `--model` mapping and excluded secondary knobs.
- `SEAM-2 blocks SEAM-3` because: Codex run wiring must consume the shared effective model-id contract, not ad hoc raw extension parsing.
- `SEAM-2 blocks SEAM-4` because: Claude run wiring must consume the shared effective model-id contract, not ad hoc raw extension parsing.
- `SEAM-2 blocks SEAM-5` because: tests must pin capability advertising and normalization behavior, including R0 ordering.
- `SEAM-3 blocks SEAM-5` because: tests must verify the final Codex mapping and backend-error translation behavior.
- `SEAM-4 blocks SEAM-5` because: tests must verify the final Claude mapping and exclusion of `--fallback-model`.

## Critical path

`SEAM-1 (contract)` → `SEAM-2 (advertising + normalization)` → `SEAM-3/SEAM-4 (backend mapping)` → `SEAM-5 (tests)`

## Parallelization notes / conflict-safe workstreams

- **WS-SPEC**: SEAM-1 docs-first contract alignment under `docs/specs/universal-agent-api/`.
- **WS-NORMALIZE**: SEAM-2 capability advertising + normalization extraction in `crates/agent_api/src/backends/**`.
- **WS-CODEX**: SEAM-3 Codex request mapping and runtime error translation.
- **WS-CLAUDE**: SEAM-4 Claude Code request mapping and runtime error translation.
- **WS-TESTS**: SEAM-5 regression coverage; can start with R0 + schema-validation harness tests once SEAM-1 is stable.
- **WS-INT (Integration)**: run `make test` / `make preflight`, validate advertised capabilities, and confirm no raw stderr leakage in backend failures.

## Pinned decisions / resolved threads

- **Opaque id posture**: v1 standardizes the request surface, not a shared cross-backend model catalog. See MS-C01/MS-C03.
- **Absence means backend default**: missing key never synthesizes a model override. See MS-C02.
- **No secondary routing by implication**: this key cannot imply fallback-model, reasoning tuning, or policy changes. See MS-C06/MS-C07.
- **Runtime unknown-model handling stays backend-owned**: safe `Backend` error translation is required, but a universal rejection string is not. See MS-C04.
