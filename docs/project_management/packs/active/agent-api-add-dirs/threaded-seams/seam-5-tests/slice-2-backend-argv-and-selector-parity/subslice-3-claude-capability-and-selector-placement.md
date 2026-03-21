### S2c — Claude capability + selector-branch `--add-dir` placement

- **User/system value**: proves the Claude backend advertises `agent_api.exec.add_dirs.v1` and
  keeps one variadic `--add-dir <DIR...>` group in the exact selector-branch position across
  fresh, resume, and fork flows.
- **Scope (in/out)**:
  - In:
    - Claude capability publication coverage for `agent_api.exec.add_dirs.v1`.
    - Fresh-run argv placement coverage for one variadic `--add-dir <DIR...>` group.
    - Resume selector `"last"` / `"id"` placement coverage after `--model` and before
      `--continue` / `--resume`.
    - Fork selector `"last"` / `"id"` placement coverage after `--model` and before
      `--fork-session` / `--resume`.
    - Absence semantics: omitted extension key means no emitted `--add-dir`.
    - Final `--verbose` ordering after the add-dir group.
  - Out:
    - Codex exec/resume repeated-pair coverage (`S2a`).
    - Codex fork rejection boundary coverage (`S2b`).
    - Claude runtime rejection parity after a handle is returned (`S3`).
- **Acceptance criteria**:
  - Claude built-in capabilities include `agent_api.exec.add_dirs.v1`.
  - Fresh-run emits one variadic `--add-dir <DIR...>` group in normalized order.
  - Resume `"last"` / `"id"` and fork `"last"` / `"id"` keep that group after `--model` and
    before branch-specific selector flags.
  - Absent inputs emit no `--add-dir`.
  - The final `--verbose` remains later in argv than the add-dir group on every covered branch.
  - A regression that duplicates the group, shifts it after branch flags, or omits it for one
    selector branch fails deterministically.
- **Dependencies**:
  - SEAM-4 Claude mapping completion (`AD-C06`)
  - `AD-C04` selector parity and `AD-C07` absence semantics from the threading registry
  - Shared normalization truth from `S1`
- **Verification**:
  - `cargo test -p agent_api --all-features claude_code`
- **Rollout/safety**:
  - Test-only sub-slice. Separate fresh/resume/fork selector branches so failures point at the
    exact placement branch that drifted.

#### S2c.T1 — Add Claude capability + selector-branch argv placement tests

- **Outcome**: Claude backend-local coverage pins capability advertisement and the exact variadic
  add-dir group placement across fresh, resume, and fork selector branches.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`
  - `crates/agent_api/src/backends/claude_code/tests/**`
  - `crates/agent_api/src/bin/fake_claude_stream_json_agent_api.rs`
  - Evidence-only input: `crates/agent_api/src/backends/claude_code/harness.rs`

Checklist:
- Implement:
  - Add the Claude capability assertion for `agent_api.exec.add_dirs.v1`.
  - Add fresh-run placement coverage for a single variadic `--add-dir <DIR...>` group.
  - Add resume `"last"` / `"id"` and fork `"last"` / `"id"` placement tests as separate cases.
  - Keep absence behavior backend-local so omitted keys emit no Claude add-dir flags.
- Test:
  - Run `cargo test -p agent_api --all-features claude_code`.
- Validate:
  - Confirm the add-dir group stays after `--model` and before selector flags on every branch.
  - Confirm the final `--verbose` still comes after the add-dir group.
  - Confirm the fake Claude stream scenarios catch duplicated or misplaced add-dir groups.
