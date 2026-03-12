# Threading — Universal extra context roots (`agent_api.exec.add_dirs.v1`)

This section makes coupling explicit: contracts/interfaces, dependency edges, and sequencing.

## Contract registry

- **AD-C01 — Core add-dir extension key**
  - **Type**: schema
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-2/3/4/5
  - **Definition**: `agent_api.exec.add_dirs.v1` is a closed object schema with required
    `dirs: string[]`, `dirs.len()` in `1..=16`, and per-entry trimmed byte bound `<= 1024`.

- **AD-C02 — Effective add-dir set algorithm**
  - **Type**: config
  - **Owner seam**: SEAM-2
  - **Consumers**: SEAM-3/4/5
  - **Definition**: the wrapper computes one effective directory list by trimming entries,
    resolving relatives against the effective working directory, lexically normalizing,
    verifying `exists && is_dir`, and deduplicating while preserving first occurrence order.

- **AD-C03 — Safe error posture**
  - **Type**: policy
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-2/3/4/5
  - **Definition**: `InvalidRequest` messages for this key are stable/testable and MUST NOT echo
    raw path values; runtime failures surface as safe/redacted backend errors.

- **AD-C04 — Session-flow parity**
  - **Type**: integration
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-3/4/5
  - **Definition**: accepted add-dir inputs are valid for new-session, resume, and fork flows.
    A backend must honor the same effective add-dir set or fail safely; it must not silently
    ignore accepted inputs for session-based flows.

- **AD-C05 — Codex argv mapping**
  - **Type**: integration
  - **Owner seam**: SEAM-3
  - **Consumers**: SEAM-5
  - **Definition**: Codex receives one repeated `--add-dir <DIR>` pair per normalized unique
    directory, in order.

- **AD-C06 — Claude Code argv mapping**
  - **Type**: integration
  - **Owner seam**: SEAM-4
  - **Consumers**: SEAM-5
  - **Definition**: Claude Code receives one variadic `--add-dir <DIR...>` group containing the
    normalized unique directories, in order.

- **AD-C07 — Absence semantics**
  - **Type**: policy
  - **Owner seam**: SEAM-1
  - **Consumers**: SEAM-2/3/4/5
  - **Definition**: when the key is absent, no backend synthesizes extra directories and no
    `--add-dir` argv is emitted.

## Dependency graph (text)

- `SEAM-1 blocks SEAM-2` because: the shared normalizer must implement the already-pinned v1
  schema, normalization, and safe-error rules.
- `SEAM-2 blocks SEAM-3` because: Codex support should consume the shared normalized directory set
  instead of inventing backend-local path semantics.
- `SEAM-2 blocks SEAM-4` because: Claude Code support should consume the same shared normalized
  directory set instead of inventing backend-local path semantics.
- `SEAM-3 blocks SEAM-5` because: tests must pin Codex capability advertising, argv order, and
  session-flow behavior.
- `SEAM-4 blocks SEAM-5` because: tests must pin Claude Code capability advertising, argv order,
  and session-flow behavior.

## Critical path

`SEAM-1 (contract)` → `SEAM-2 (shared normalizer)` → `SEAM-3/SEAM-4 (backend mapping)` →
`SEAM-5 (tests)`

## Integration points

- **Run extension gate**: `backend_harness::normalize_request()` must fail closed on unsupported
  keys before any add-dir value parsing happens.
- **Effective working directory handoff**: the shared normalizer and each backend’s spawn path
  must agree on the same working directory source.
- **Session selectors**: resume/fork parsing stays orthogonal, but accepted add-dir inputs must
  survive into those flows.
- **Wrapper crate parity**: `codex::CodexClientBuilder` and `claude_code::ClaudePrintRequest`
  already expose add-dir surfaces; backend seams wire the normalized list into them.

## Parallelization notes / conflict-safe workstreams

- **WS-CONTRACT**: SEAM-1 (`extensions-spec.md` confirmation + pack contract).
- **WS-NORMALIZE**: SEAM-2 (shared normalizer + reusable validation/resolution helpers).
- **WS-CODEX**: SEAM-3 (Codex capability + policy + exec/resume/fork mapping).
- **WS-CLAUDE**: SEAM-4 (Claude capability + policy + print/resume/fork mapping).
- **WS-TESTS**: SEAM-5 (shared normalizer tests plus backend capability/mapping/session tests).
- **WS-INT (Integration)**: end-to-end validation and `make preflight` after the seams land.

## Pinned decisions / resolved threads

- **Relative paths are allowed** and resolve against the effective working directory.
- **No containment rule** is imposed for v1.
- **Lexical normalization only**: no shell expansion, env expansion, canonicalization, or symlink
  resolution requirement.
- **Dedup is not an error**: duplicates collapse after normalization while preserving order.
