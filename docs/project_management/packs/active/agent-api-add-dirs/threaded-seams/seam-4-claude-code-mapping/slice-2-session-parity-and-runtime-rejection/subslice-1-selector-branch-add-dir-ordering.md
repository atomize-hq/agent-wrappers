### S2a — Selector-branch add-dir ordering parity

- **User/system value**: accepted add-dir inputs stay present and correctly ordered across all
  Claude resume/fork selector branches without mixing runtime rejection work into the same session.
- **Scope (in/out)**:
  - In:
    - Resume selector `"last"` and `"id"` add-dir placement.
    - Fork selector `"last"` and `"id"` add-dir placement.
    - Backend-local ordered-subsequence assertions covering those branches.
    - Canonical Claude mapping doc text limited to branch-specific add-dir placement.
  - Out:
    - Add-dir runtime rejection classification and terminal error propagation.
    - Shared fake-runtime scenario ids and cross-backend fixture work owned by SEAM-5.
    - Capability-matrix regeneration.
- **Acceptance criteria**:
  - Resume `"last"` emits `--continue` only after the add-dir group.
  - Resume `"id"` emits `--resume <ID>` only after the add-dir group.
  - Fork `"last"` emits `--continue --fork-session` only after the add-dir group.
  - Fork `"id"` emits `--fork-session --resume <ID>` only after the add-dir group.
  - Each selector branch has an explicit ordered-subsequence assertion that includes add dirs.
- **Dependencies**:
  - Blocked by: `S1`, `AD-C04`, `AD-C06`
  - Unblocks: `S2c`, SEAM-5 selector-branch fixture derivation
- **Verification**:
  - Backend-local selector-branch ordering assertions stay green.
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Reuse the single shared `print_req.add_dirs(...)` insertion from `S1`; do not add
    branch-specific flag emission logic.
  - Keep the add-dir group before selector tokens and before the final `--verbose` token.

#### S2.T1 — Preserve add-dir placement across Claude resume and fork selector branches

- **Outcome**: the accepted normalized directory set remains present and correctly ordered across
  resume/fork selector `"last"` and `"id"` flows.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`
  - `docs/specs/claude-code-session-mapping-contract.md`

Checklist:
- Implement:
  - Verify branch-specific `print_req` construction preserves the shared add-dir group.
  - Keep the existing selector token ordering pinned:
    - resume `"last"` -> `--continue`
    - resume `"id"` -> `--resume <ID>`
    - fork `"last"` -> `--continue --fork-session`
    - fork `"id"` -> `--fork-session --resume <ID>`
- Test:
  - Add ordered-subsequence assertions for resume/fork `"last"` and `"id"` branches.
- Validate:
  - Compare emitted branch subsequences against the canonical doc text.
  - Confirm add dirs remain before the final `--verbose` token.
