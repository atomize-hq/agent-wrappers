# Threaded Seam Decomposition — SEAM-4 Claude Code backend support

Pack: `docs/project_management/packs/active/agent-api-add-dirs/`

Inputs:
- Seam brief: `docs/project_management/packs/active/agent-api-add-dirs/seam-4-claude-code-mapping.md`
- Threading (authoritative): `docs/project_management/packs/active/agent-api-add-dirs/threading.md`
- Scope brief: `docs/project_management/packs/active/agent-api-add-dirs/scope_brief.md`
- Canonical backend contract: `docs/specs/claude-code-session-mapping-contract.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-4
- **Name**: Claude Code `agent_api.exec.add_dirs.v1` backend support
- **Goal / value**: let Claude Code fresh-run, resume, and fork flows consume the shared
  normalized add-dir set and map it to the pinned variadic `--add-dir <DIR...>` CLI contract
  without silently dropping accepted inputs.
- **Type**: platform / integration
- **Scope**
  - In:
    - Advertise `agent_api.exec.add_dirs.v1` from the built-in Claude backend once the seam lands.
    - Add the key to Claude supported-extension allowlists so R0 gating is authoritative.
    - Thread the SEAM-2 normalized `Vec<PathBuf>` through Claude policy extraction instead of
      rereading raw request extensions in spawn code.
    - Map the normalized list into exactly one `--add-dir <DIR...>` argv group using
      `ClaudePrintRequest`.
    - Preserve the accepted directory set across fresh-run, resume selector `"last"`, resume
      selector `"id"`, fork selector `"last"`, and fork selector `"id"`.
    - Honor the pinned safe runtime rejection posture for accepted add-dir inputs on Claude
      surfaces that already returned a handle.
    - Update `docs/specs/claude-code-session-mapping-contract.md` so the backend-owned mapping is
      explicit and testable.
  - Out:
    - Core key semantics, safe `InvalidRequest` messages, and session orthogonality rules
      (SEAM-1).
    - Shared normalization, filesystem validation, deduplication, and effective-working-dir
      resolution helper behavior (SEAM-2).
    - Cross-backend regression matrix, fake runtime scenario inventory, capability-matrix
      regeneration, and final integration closeout (SEAM-5).
- **Touch surface**:
  - `crates/agent_api/src/backends/claude_code/mod.rs`
  - `crates/agent_api/src/backends/claude_code/backend.rs`
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`
  - `docs/specs/claude-code-session-mapping-contract.md`
- **Verification**:
  - Local Claude backend unit coverage for capability advertising, policy extraction, and argv
    ordering stays green.
  - `cargo test -p agent_api claude_code`
- **Threading constraints**
  - Upstream blockers: SEAM-1, SEAM-2
  - Downstream blocked seams: SEAM-5
  - Contracts produced (owned): AD-C06
  - Contracts consumed: AD-C01, AD-C02, AD-C03, AD-C04, AD-C07

## Slicing Strategy

**Contract-first / dependency-first**.

SEAM-4 owns the Claude mapping contract `AD-C06` and blocks SEAM-5, so the first slice publishes
the backend-visible contract in code and docs as a thin, testable root-flags mapping. The second
slice extends that same contract across resume/fork branches and the pinned runtime rejection
posture without pulling SEAM-5’s cross-backend fixture work into this seam.

## Vertical Slices

- **S1 — Capability, policy extraction, and root-flags mapping**
  - File:
    `docs/project_management/packs/active/agent-api-add-dirs/threaded-seams/seam-4-claude-code-mapping/slice-1-capability-policy-and-root-flags.md`
- **S2 — Session-branch parity and runtime rejection conformance**
  - File:
    `docs/project_management/packs/active/agent-api-add-dirs/threaded-seams/seam-4-claude-code-mapping/slice-2-session-parity-and-runtime-rejection.md`

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `AD-C06 — Claude Code argv mapping`
    - Definition: Claude Code receives one variadic `--add-dir <DIR...>` group containing the
      normalized unique directories, in order, after any accepted `--model` pair and before
      `--continue` / `--fork-session` / `--resume` and the final `--verbose` token.
    - Where it lives: implemented in `crates/agent_api/src/backends/claude_code/**` and pinned in
      `docs/specs/claude-code-session-mapping-contract.md`.
    - Produced by: `S1` establishes the root-flags mapping; `S2` completes selector-branch and
      runtime-rejection conformance.
- **Contracts consumed**:
  - `AD-C01 — Core add-dir extension key` (SEAM-1)
    - Consumed by: `S1.T1` and `S1.T2` to advertise and extract
      `agent_api.exec.add_dirs.v1` without inventing backend-local schema rules.
  - `AD-C02 — Effective add-dir set algorithm` (SEAM-2)
    - Consumed by: `S1.T2` to call the shared normalizer, carry `Vec<PathBuf>` on Claude policy,
      and avoid rereading raw request extensions downstream.
  - `AD-C03 — Safe error posture` (SEAM-1)
    - Consumed by: `S1.T2` for invalid-input failures and `S2.T2` for safe/redacted runtime
      rejection messages.
  - `AD-C04 — Session-flow parity` (SEAM-1)
    - Consumed by: `S2.T1` and `S2.T2` to preserve accepted add-dir inputs across resume/fork and
      keep the runtime-failure posture branch-consistent.
  - `AD-C07 — Absence semantics` (SEAM-1)
    - Consumed by: `S1.T3` so absent-key flows still emit no `--add-dir` argv.
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-4`: this plan assumes the key schema, safe invalid-message templates, and
    session-parity rules are already pinned in the normative specs before Claude wiring lands.
  - `SEAM-2 blocks SEAM-4`: `S1.T2` depends on the shared `normalize_add_dirs_v1(...)` helper and
    the effective-working-dir handoff owned by the shared normalizer seam.
  - `SEAM-4 blocks SEAM-5`: `S1` and `S2` deliver the concrete Claude mapping behavior that
    SEAM-5 later pins in exhaustive regression coverage.
- **Parallelization notes**:
  - What can proceed now:
    - Draft doc updates and backend-local ordering assertions while SEAM-2 finalizes the helper
      signature, because the Claude touch surface is isolated from Codex.
    - Prepare capability and policy-struct edits in `claude_code/**` as long as the code consumes
      the shared helper rather than duplicating normalization logic.
  - What must wait:
    - Final code wiring must wait for SEAM-2’s exported helper + effective-working-dir contract.
    - SEAM-5’s fake-runtime scenarios, capability-matrix regeneration, and `make preflight`
      closeout stay out of this seam.
