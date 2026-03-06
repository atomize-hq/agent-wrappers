# Threaded Seam Decomposition — SEAM-5 MCP management regression tests

Pack: `docs/project_management/packs/active/agent-api-mcp-management/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-mcp-management/seam-5-tests.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-mcp-management/threading.md`
- Canonical spec (normative once approved): `docs/specs/universal-agent-api/mcp-management-spec.md`
- Canonical core contract (normative): `docs/specs/universal-agent-api/contract.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-5
- **Name**: MCP management regression tests
- **Goal / value**: Prevent drift in the universal MCP management surface by verifying:
  - capability gating + safe default advertising posture,
  - request validation + validate-before-spawn,
  - process context precedence (working_dir/timeout/env),
  - command execution semantics + error taxonomy (`Ok(output)` vs `Err(Backend)`),
  - stdout/stderr output bounds + deterministic truncation,
  - built-in backend mappings (Codex + Claude Code) and pinned manifest drift behavior.
- **Type**: integration (verification)
- **Scope**
  - In:
    - Hermetic fake-binary integration tests for `mcp_list/get/add/remove` under isolated homes (no user-state mutation).
    - Target-aware tests for Claude’s pinned subcommand availability (`win32-x64` only for `get/add/remove`).
    - Regression tests for the **non-run boundary** (MCP management must not be modeled as run extensions).
    - Optional live smoke tests that are opt-in and `#[ignore]` by default (per the canonical spec verification policy).
  - Out:
    - End-to-end tests requiring a real networked MCP server.
    - Tests that assert a universal structured MCP config schema (v1 returns bounded stdout/stderr only).
- **Touch surface**:
  - `crates/agent_api/tests/**` (integration tests + shared test support)
  - `crates/agent_api/src/bin/**` (hermetic fake `codex` / `claude` binaries used by tests)
- **Verification**:
  - `cargo test -p agent_api --all-features --test c5_mcp_management_v1 -- --nocapture`
  - `make test` (workspace, all targets/features)
- **Threading constraints**
  - Upstream blockers: SEAM-1, SEAM-2, SEAM-3, SEAM-4
  - Downstream blocked seams: none (enables WS-INT confidence + `make preflight`)
  - Contracts produced (owned): none (verification seam only)
  - Contracts consumed: MM-C01, MM-C02, MM-C03, MM-C04, MM-C05, MM-C06, MM-C07, MM-C08, MM-C09

## Slicing Strategy

**Dependency-first / risk-first** within the seam:

1) Land a hermetic fake-binary harness + cross-backend “capability + non-run boundary” regressions first (offline; no
   real upstream binaries; no user-state mutation).
2) Pin Codex MCP management mapping end-to-end using the fake `codex` binary.
3) Pin Claude Code mapping end-to-end using the fake `claude` binary, with explicit target gating for `win32-x64`-only
   operations, plus `Url.bearer_token_env_var` rejection behavior.

## Vertical Slices

- **S1 — Hermetic fake-binary harness + capability/non-run regressions**
  - File: `docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-5-tests/slice-1-hermetic-harness.md`
- **S2 — Codex MCP integration tests (argv/env/isolation/bounds/drift)**
  - File: `docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-5-tests/slice-2-codex-integration.md`
- **S3 — Claude Code MCP integration tests (target-aware + bearer-token rule)**
  - File: `docs/project_management/packs/active/agent-api-mcp-management/threaded-seams/seam-5-tests/slice-3-claude-integration.md`

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - None (SEAM-5 is verification-only).
- **Contracts consumed**:
  - `MM-C01 — MCP management capability ids (v1)` (SEAM-1): tests assert capability advertising + gateway fail-closed behavior.
  - `MM-C02 — Non-run command boundary` (SEAM-1): tests ensure MCP management cannot be invoked as run extensions and does not
    enter the run event pipeline.
  - `MM-C03 — Process context contract` (SEAM-1): tests pin working_dir/timeout/env precedence in spawned management commands.
  - `MM-C04 — Output bounds contract` (SEAM-1): tests pin 65,536-byte stdout/stderr budgets + suffix + flags end-to-end.
  - `MM-C05 — Add transport typing (no argv pass-through)` (SEAM-1): tests pin typed transports for `mcp_add` (no extra args).
  - `MM-C06 — Safe default advertising (write ops)` (SEAM-2): tests pin default posture and `allow_mcp_write` gating.
  - `MM-C07 — Isolated home support` (SEAM-2): tests pin isolated-home behavior + request env overrides winning.
  - `MM-C08 — Codex MCP mapping contract` (SEAM-3): tests pin Codex argv mapping + drift behavior.
  - `MM-C09 — Claude MCP mapping contract` (SEAM-4): tests pin Claude argv mapping + target gating + bearer-token rejection.
- **Dependency edges honored**:
  - `SEAM-2 blocks SEAM-5`: S1 pins safe advertising + isolation behavior; S2/S3 assume enablement exists.
  - `SEAM-3 blocks SEAM-5`: S2 pins Codex mapping after the mapping seam lands.
  - `SEAM-4 blocks SEAM-5`: S3 pins Claude mapping after the mapping seam lands.
- **Parallelization notes**:
  - What can proceed now:
    - S1 harness code (fake binaries + test support) can be implemented as soon as `agent_api` builds in a branch with SEAM-1.
  - What must wait:
    - S2 assertions should be finalized only after SEAM-3 lands (avoid duplicating/predicting mapping logic).
    - S3 assertions should be finalized only after SEAM-4 lands (and must remain target-aware for pinned availability).

## Integration suggestions (explicitly out-of-scope for SEAM-5 tasking)

- After S1–S3 land, WS-INT should run `make preflight` per `threading.md` critical path.
- If adding live smoke tests, keep them `#[ignore]` + gated by `AGENT_API_MCP_LIVE=1` so CI remains offline/deterministic.

