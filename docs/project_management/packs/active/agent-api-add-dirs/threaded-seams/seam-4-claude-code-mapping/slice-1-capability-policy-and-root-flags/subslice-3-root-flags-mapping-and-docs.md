### S1c — Fresh-run root-flags mapping and Claude contract pinning

- **User/system value**: fresh Claude runs emit one pinned variadic `--add-dir <DIR...>` group in
  the root-flags region, and the Claude-owned contract text states that ordering unambiguously.
- **Scope (in/out)**:
  - In:
    - Wire `ClaudePrintRequest::add_dirs(...)` into the fresh-run path exactly once.
    - Preserve the pinned order around `--model`, `--add-dir`, final `--verbose`, and prompt.
    - Keep absent-key flows on the no-flag path.
    - Update the seam-local Claude mapping contract and backend-local ordering assertions.
  - Out:
    - Resume/fork selector-branch parity.
    - Post-handle runtime rejection behavior.
    - Capability-matrix regeneration and SEAM-5 integration fixtures.
- **Acceptance criteria**:
  - Fresh-run argv contains exactly one `--add-dir` token followed by all normalized directories
    in order.
  - No `--add-dir` token appears when the key is absent.
  - Any accepted `--model` pair remains earlier in argv order than the add-dir group.
  - The add-dir group remains earlier than the final `--verbose` token and prompt.
  - `docs/specs/claude-code-session-mapping-contract.md` describes the same fresh-run ordering.
- **Dependencies**:
  - Consumes `AD-C06` and `AD-C07`.
  - Depends on `S1b` because the argv layer must consume policy state rather than raw request
    extensions.
- **Verification**:
  - Backend-local argv ordering assertions
  - `cargo test -p agent_api claude_code`
- **Rollout/safety**:
  - Preserve existing `--permission-mode`, external-sandbox, and model ordering while inserting
    add dirs.
  - Keep the contract doc update in the same change as the argv-order assertions.

#### S1c.T1 — Emit one variadic add-dir group in fresh-run argv and pin it in docs

- **Outcome**: Claude fresh-run command assembly and the canonical mapping contract express the
  same root-flags truth with backend-local tests guarding drift.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backends/claude_code/tests/backend_contract.rs`
  - `docs/specs/claude-code-session-mapping-contract.md`

Checklist:
- Implement:
  - Call `ClaudePrintRequest::add_dirs(...)` once with the normalized list from policy state.
  - Keep absent-key flows on the no-flag path instead of emitting an empty `--add-dir`.
  - Update the Claude mapping contract with the fresh-run root-flags placement rule.
- Test:
  - Add backend-local ordering assertions for `--model`, `--add-dir`, final `--verbose`, and
    prompt placement.
  - Add a no-key assertion proving no `--add-dir` token is emitted.
- Validate:
  - Compare emitted argv order directly against the contract text.
  - Confirm this sub-slice does not pull in resume/fork parity or runtime-rejection scope.
