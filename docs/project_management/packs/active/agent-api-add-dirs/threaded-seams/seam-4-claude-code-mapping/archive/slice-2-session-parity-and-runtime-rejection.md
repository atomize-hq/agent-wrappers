# S2 — Session-branch parity and runtime rejection conformance

- **User/system value**: accepted add-dir inputs survive every Claude session selector branch, and
  any supported Claude surface that later rejects those inputs fails in the pinned safe,
  observable way instead of silently dropping them.
- **Scope (in/out)**:
  - In:
    - Resume selector `"last"` and `"id"` keep the same accepted add-dir group.
    - Fork selector `"last"` and `"id"` keep the same accepted add-dir group.
    - Claude runtime rejection after handle creation emits exactly one terminal error event with a
      matching completion error message.
    - Canonical Claude session-mapping doc updates for branch-specific add-dir placement and
      runtime rejection parity.
  - Out:
    - Shared fake-runtime scenario ids and exhaustive cross-backend regression coverage (SEAM-5).
    - Any Codex-specific fork rejection behavior.
- **Acceptance criteria**:
  - Resume `"last"` emits `--continue` only after the add-dir group.
  - Resume `"id"` emits `--resume <ID>` only after the add-dir group.
  - Fork `"last"` emits `--continue --fork-session` only after the add-dir group.
  - Fork `"id"` emits `--fork-session --resume <ID>` only after the add-dir group.
  - A post-handle Claude runtime rejection for accepted add dirs emits one terminal
    `AgentWrapperEventKind::Error` event and surfaces the same safe/redacted message through
    completion.
- **Dependencies**:
  - Blocked by: S1, SEAM-1 (`AD-C03`, `AD-C04`)
  - Unblocks: SEAM-5
- **Verification**:
  - Backend-local selector-branch ordering assertions
  - Backend-local event/completion parity assertions
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Preserve selection-failure translation and generic non-zero redaction behavior while adding the
    add-dir-specific runtime rejection posture.
  - Keep all branch ordering rules in the canonical Claude mapping doc so SEAM-5 tests pin one
    truth.

#### S2.T1 — Preserve add-dir placement across Claude resume and fork selector branches

- **Outcome**: the accepted normalized directory set remains present and correctly ordered across
  resume/fork selector `"last"` and `"id"` flows.
- **Inputs/outputs**:
  - Inputs: `AD-C04`, `AD-C06`, session selector handling in
    `crates/agent_api/src/backends/claude_code/harness.rs`
  - Outputs: branch-safe `print_req` construction plus canonical subsequence text in
    `docs/specs/claude-code-session-mapping-contract.md`
- **Implementation notes**:
  - Reuse the single `print_req.add_dirs(...)` root-flags insertion from S1; do not special-case
    individual selector branches.
  - Verify the existing branch order remains pinned:
    - resume `"last"` -> `--continue`
    - resume `"id"` -> `--resume <ID>`
    - fork `"last"` -> `--continue --fork-session`
    - fork `"id"` -> `--fork-session --resume <ID>`
  - Keep the add-dir group before every selector token and before the final `--verbose` token.
- **Acceptance criteria**:
  - Each selector branch has an explicit ordered-subsequence assertion that includes add dirs.
  - The canonical Claude contract doc names all four selector branches with add-dir placement.
- **Test notes**:
  - Add backend-local ordering assertions only; SEAM-5 owns the exhaustive fake-stream coverage.
- **Risk/rollback notes**:
  - The main regression risk is changing relative order between `--fork-session` and `--resume`.

Checklist:
- Implement: verify branch-specific `print_req` construction preserves the shared add-dir group.
- Test: add ordering assertions for resume/fork `"last"` and `"id"` branches.
- Validate: compare emitted branch subsequences against the canonical doc text.
- Cleanup: remove any branch comments that imply add dirs are fresh-run-only.

#### S2.T2 — Implement safe runtime rejection parity for accepted add-dir inputs

- **Outcome**: when Claude accepts add dirs, returns a handle, and later rejects them at runtime,
  the backend emits exactly one terminal safe error event and completes with the same safe/redacted
  backend message.
- **Inputs/outputs**:
  - Inputs: `AD-C03`, `AD-C04`, Claude event/completion plumbing in
    `crates/agent_api/src/backends/claude_code/harness.rs` and
    `crates/agent_api/src/backends/claude_code/mapping.rs`
  - Outputs: add-dir-specific runtime rejection classification and a pinned safe message path
    suitable for SEAM-5 fixture coverage
- **Implementation notes**:
  - Keep selection-failure translation separate; add-dir runtime rejection is not a selector miss.
  - Reuse the terminal-error event path so the events stream closes after one
    `AgentWrapperEventKind::Error`.
  - Use the backend-owned safe/redacted message `add_dirs rejected by runtime`; do not leak raw
    stdout/stderr.
- **Acceptance criteria**:
  - Handle-returning Claude surfaces produce one terminal error event carrying the same message
    later surfaced through completion.
  - Generic non-zero exit handling remains reserved for unrelated failures.
- **Test notes**:
  - Add backend-local parity assertions around terminal error emission and completion matching.
  - Leave fake runtime scenario ids such as `add_dirs_runtime_rejection_*` to SEAM-5.
- **Risk/rollback notes**:
  - Do not route add-dir runtime failures through the generic redaction path if parity would be
    lost.

Checklist:
- Implement: add add-dir-specific runtime rejection classification and terminal error propagation.
- Test: add local parity assertions for error-event/completion message matching.
- Validate: confirm only one terminal `Error` event is emitted before stream close.
- Cleanup: document the safe message string in one place to avoid drift.

#### S2.T3 — Finalize Claude-owned conformance docs and seam-local regression hooks

- **Outcome**: the Claude backend’s normative mapping doc and local regression hooks reflect the
  final AD-C06 truth without absorbing SEAM-5’s shared integration work.
- **Inputs/outputs**:
  - Inputs: completed S1/S2 behavior, `docs/specs/claude-code-session-mapping-contract.md`,
    `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`
  - Outputs: finalized add-dir placement + runtime-parity clauses in the doc and backend-local
    assertions that guard against accidental drift
- **Implementation notes**:
  - Keep this slice focused on Claude-owned conformance surfaces only.
  - Do not regenerate `docs/specs/universal-agent-api/capability-matrix.md` here; that belongs to
    SEAM-5 when both built-in backends advertise the key.
  - If a backend-contract assertion is added, keep it scoped to Claude routing/ordering and avoid
    duplicating cross-backend checks.
- **Acceptance criteria**:
  - The canonical Claude mapping doc is sufficient for SEAM-5 to derive branch-specific tests.
  - Backend-local regression hooks fail if add-dir ordering or runtime error posture drifts.
- **Test notes**:
  - Run `cargo test -p agent_api claude_code` after doc/code updates land together.
- **Risk/rollback notes**:
  - Avoid spreading the same ordering truth across multiple docs or helper comments.

Checklist:
- Implement: finalize doc text and any minimal backend-contract assertions needed for drift guard.
- Test: run the Claude backend test slice after the doc-backed code changes land.
- Validate: ensure SEAM-5 still owns capability-matrix regen and exhaustive fake-runtime coverage.
- Cleanup: remove stale planning notes once the canonical doc is updated.
