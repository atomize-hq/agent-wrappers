# SEAM-2 — Shared `agent_api` add-dir normalizer

- **Name**: shared add-dir request parsing / validation / resolution helper
- **Type**: integration
- **Goal / user value**: compute the effective add-dir set once so Codex and Claude Code consume
  the same validated directory list and error posture.

## Scope

- In:
  - Introduce shared code that:
    - parses `agent_api.exec.add_dirs.v1`,
    - enforces the closed schema and bounds,
    - resolves relative paths from the effective working directory,
    - lexically normalizes,
    - validates `exists && is_dir`,
    - deduplicates while preserving order,
    - returns safe/testable errors without path leaks.
  - Define the shared output shape consumed by backend seams, for example `Vec<PathBuf>`.
- Out:
  - Backend capability advertising.
  - Backend-specific CLI argv emission.

## Primary interfaces (contracts)

- **Normalizer input contract**
  - **Inputs**:
    - raw extension value
    - effective working directory
  - **Outputs**:
    - normalized unique directory list or `InvalidRequest`

- **Backend-consumption contract**
  - **Inputs**:
    - normalized directory list
  - **Outputs**:
    - backend policy/spawn layers can map it without re-validating semantics

## Key invariants / rules

- The shared helper is the only place that decides trimming, path resolution, normalization, and
  dedup behavior.
- Errors identify the failing field or index using the exact templates
  `invalid agent_api.exec.add_dirs.v1`, `invalid agent_api.exec.add_dirs.v1.dirs`, or
  `invalid agent_api.exec.add_dirs.v1.dirs[<i>]`, and never the raw path text.
- The helper must not create a new working-directory precedence ladder that diverges from actual
  backend execution behavior.

## Dependencies

- Blocks: SEAM-3/4/5
- Blocked by: SEAM-1

## Touch surface

- `crates/agent_api/src/backend_harness/normalize.rs`
- `crates/agent_api/src/backends/session_selectors.rs` or a new sibling helper module under
  `crates/agent_api/src/backends/`
- `crates/agent_api/src/backend_harness/contract.rs`

## Verification

- Unit tests for:
  - non-object value rejection,
  - unknown key rejection,
  - missing `dirs`,
  - non-array `dirs`,
  - length bounds,
  - non-string entries,
  - trimmed empty entries,
  - byte-length bounds,
  - relative resolution,
  - lexical normalization,
  - dedup order preservation,
  - missing/non-directory path rejection,
  - exact safe InvalidRequest template selection with no raw path leakage in errors.

## Risks / unknowns

- **Risk**: resolving add-dirs too early could use the wrong working directory source.
- **De-risk plan**: thread the effective working directory explicitly into the helper from the
  backend adapter layer that already owns run defaults.

## Rollout / safety

- Shared helper should be introduced first so backend seams can consume one pinned implementation.
