# Threading — External sandbox execution policy (dangerous)

This section makes coupling explicit: contracts/interfaces, dependency edges, and sequencing.

## Contract registry

- **ES-C01 — External sandbox execution policy extension key**
  - **Type**: config (core extension key)
  - **Definition**: `agent_api.exec.external_sandbox.v1` is a boolean extension key, validated
    before spawn, that requests a backend to relax internal guardrails because the host provides
    isolation externally.
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-2/3/4/5

- **ES-C02 — Non-interactive invariant (external sandbox mode)**
  - **Type**: policy
  - **Definition**: When `agent_api.exec.external_sandbox.v1 == true`, the backend MUST remain
    non-interactive. If `agent_api.exec.non_interactive == false` is explicitly requested (and both
    keys are supported), the backend MUST fail before spawn with `AgentWrapperError::InvalidRequest`
    (contradictory intent).
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-3/4/5

- **ES-C03 — Safe default advertising**
  - **Type**: permission
  - **Definition**: Built-in backends MUST NOT advertise `agent_api.exec.external_sandbox.v1` by
    default; externally sandboxed hosts opt-in explicitly via backend configuration
    (`allow_external_sandbox_exec`; see `docs/specs/universal-agent-api/contract.md`).
  - **Owner seam**: SEAM-2
  - **Consumers**: SEAM-3/4/5

- **ES-C04 — Codex mapping contract**
  - **Type**: config
  - **Definition**: When enabled + requested, Codex backend mapping is pinned (canonical:
    `docs/specs/codex-external-sandbox-mapping-contract.md`; planning summary: SEAM-3):
    - exec/resume: apply `CodexClientBuilder::dangerously_bypass_approvals_and_sandbox(true)`
      (argv includes `--dangerously-bypass-approvals-and-sandbox` and excludes other safety flags).
    - fork/app-server: RPC uses:
      - `approvalPolicy="never"` (thread/fork + turn/start)
      - `sandbox="danger-full-access"` (thread/fork)
  - **Owner seam**: SEAM-3

- **ES-C05 — Claude mapping contract**
  - **Type**: config
  - **Definition**: When enabled + requested, Claude Code backend maps the key to
    `claude --print --dangerously-skip-permissions ...` and applies any additional required opt-in
    flag deterministically (pre-spawn) based on CLI capability, per
    `docs/specs/claude-code-session-mapping-contract.md`.
  - **Owner seam**: SEAM-4

- **ES-C06 — Exec-policy combination rule (external sandbox mode)**
  - **Type**: policy
  - **Definition**: When `agent_api.exec.external_sandbox.v1 == true`, the request MUST NOT include
    any backend-scoped exec-policy keys under `backend.<agent_kind>.exec.*` (ambiguous precedence).
    If such a key is present (and supported per the extensions registry R0 gate), the backend MUST
    fail before spawn with `AgentWrapperError::InvalidRequest`. If such a key is present but is not
    supported, the backend MUST fail-closed as `AgentWrapperError::UnsupportedCapability` per R0
    and MUST NOT evaluate this contradiction rule. (Canonical:
    `docs/specs/universal-agent-api/extensions-spec.md`.)
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-3/4/5

- **ES-C07 — Claude allow-flag preflight (external sandbox mode)**
  - **Type**: integration
  - **Definition**: Claude Code allow-flag support MUST be determined pre-spawn via a deterministic
    `claude --help` preflight (cached) and MUST NOT use a spawn+retry loop. (Canonical:
    `docs/specs/claude-code-session-mapping-contract.md`.)
  - **Owner seam**: SEAM-4
  - **Consumers**: SEAM-5

## Dependency graph (text)

- `SEAM-1 blocks SEAM-2` because: backend enablement needs the final key semantics and contradiction rules.
- `SEAM-2 blocks SEAM-3` because: Codex mapping must only be reachable behind explicit opt-in (no default advertising).
- `SEAM-2 blocks SEAM-4` because: Claude mapping must only be reachable behind explicit opt-in (no default advertising).
- `SEAM-3 blocks SEAM-5` because: tests must pin the final Codex mapping behavior.
- `SEAM-4 blocks SEAM-5` because: tests must pin the final Claude mapping behavior.

## Critical path

`SEAM-1 (contract)` → `SEAM-2 (enablement)` → `SEAM-3/SEAM-4 (backend mapping)` → `SEAM-5 (tests)`

## Parallelization notes / conflict-safe workstreams

- **WS-SPEC**: SEAM-1 docs-only edits under `docs/specs/universal-agent-api/`.
- **WS-ENABLEMENT**: SEAM-2 backend config + capability advertising (Codex + Claude Code backends).
- **WS-CODEX**: SEAM-3 Codex mapping + validations.
- **WS-CLAUDE**: SEAM-4 Claude mapping + validations.
- **WS-TESTS**: SEAM-5 tests (can start with harness-level ordering tests once SEAM-1 is stable).
- **WS-INT (Integration)**: run `make preflight` and validate matrix/spec conformance after merge.

## Pinned decisions / resolved threads

- **Claude allow-flag handling**: pinned to a deterministic `claude --help` preflight (cached), with
  failure before spawn when preflight cannot be performed. See ES-C07.
- **Exec-policy combination / precedence**: pinned to “reject `backend.<agent_kind>.exec.*` keys
  when `external_sandbox=true`” (after R0 support gating) to avoid ambiguous precedence in a
  dangerous surface. See ES-C06.
