### S2a — Codex capability + exec/resume `--add-dir` parity

- **User/system value**: proves the Codex backend advertises `agent_api.exec.add_dirs.v1` and
  preserves the pinned repeated-pair `--add-dir <DIR>` argv contract on exec/resume paths.
- **Scope (in/out)**:
  - In:
    - Codex capability publication coverage for `agent_api.exec.add_dirs.v1`.
    - Exec-path argv assertions for repeated `--add-dir <DIR>` pairs after any accepted
      `--model <ID>` pair.
    - Resume-path argv assertions for selectors already accepted by the shared policy layer.
    - Absence semantics: omitted extension key means no emitted `--add-dir`.
    - Normalized order preservation for accepted directories.
  - Out:
    - Codex fork selector rejection behavior (`S2b`).
    - Claude fresh/resume/fork placement coverage (`S2c`).
    - Runtime rejection after a handle is returned (`S3`).
- **Acceptance criteria**:
  - Codex built-in capabilities include `agent_api.exec.add_dirs.v1`.
  - Exec and resume emit every `--add-dir <DIR>` pair after any accepted `--model` pair.
  - Accepted directories remain in normalized first-occurrence order.
  - Absent inputs emit no `--add-dir`.
  - A regression that drops a directory, duplicates ordering incorrectly, or moves `--model` after
    the first `--add-dir` fails deterministically.
- **Dependencies**:
  - SEAM-3 Codex mapping completion (`AD-C05`)
  - `AD-C07` absence semantics from the threading registry
  - Shared normalization truth from `S1`
- **Verification**:
  - `cargo test -p agent_api --all-features codex`
- **Rollout/safety**:
  - Test-only sub-slice. Land after the SEAM-3 Codex mapping path is stable so these assertions pin
    final ordering rather than interim plumbing.

#### S2a.T1 — Add Codex capability + exec/resume argv conformance tests

- **Outcome**: Codex backend-local coverage pins capability advertisement, repeated-pair argv
  ordering, and omission semantics for accepted add-dir inputs.
- **Files**:
  - `crates/agent_api/src/backends/codex/tests/capabilities.rs`
  - `crates/agent_api/src/backends/codex/tests/**`
  - `crates/agent_api/src/bin/fake_codex_stream_exec_scenarios_agent_api.rs`
  - Evidence-only input: `crates/agent_api/src/backends/codex/exec.rs`

Checklist:
- Implement:
  - Add the Codex capability assertion for `agent_api.exec.add_dirs.v1`.
  - Extend exec/resume argv tests to assert repeated `--add-dir <DIR>` emission after any accepted
    `--model <ID>` pair.
  - Reuse normalized fixtures where helpful, but keep the argv truth backend-local.
  - Keep absence coverage explicit so omitted keys stay omission-only behavior.
- Test:
  - Run `cargo test -p agent_api --all-features codex`.
- Validate:
  - Confirm exec and resume share the same ordering contract from `AD-C05`.
  - Confirm normalized first-occurrence order is preserved.
  - Confirm the fake exec scenario fixture catches `--model` / `--add-dir` inversions.
