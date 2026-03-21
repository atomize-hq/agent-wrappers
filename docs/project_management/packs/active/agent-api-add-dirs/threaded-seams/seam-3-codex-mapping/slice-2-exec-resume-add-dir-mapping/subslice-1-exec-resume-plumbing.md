### S2a — Exec/resume plumbing into the existing Codex builder

- **User/system value**: gets the normalized add-dir list onto the exec/resume spawn path using
  the existing builder surface, so accepted directories reach Codex with deterministic argv
  ordering.
- **Scope (in/out)**:
  - In:
    - Extend the typed exec/resume request surface with `add_dirs: Vec<PathBuf>`.
    - Carry `policy.add_dirs` through `CodexHarnessAdapter::spawn(...)`.
    - Hand the typed list to the existing Codex builder before `build()`.
    - Preserve the current builder ordering guarantee that any accepted `--model <trimmed-id>`
      pair stays earlier in argv than the first emitted `--add-dir`.
  - Out:
    - Capability advertising and shared normalization (`S1`).
    - Any new builder feature work inside `crates/codex`; this sub-slice assumes the existing
      `add_dirs(...)` surface remains the emission owner.
    - Contract-doc wording updates (`S2b`).
- **Acceptance criteria**:
  - `ExecFlowRequest` carries a typed `Vec<PathBuf>` add-dir list on both fresh exec and resume
    flows.
  - `CodexHarnessAdapter::spawn(...)` passes through `policy.add_dirs` rather than re-reading raw
    extension payloads.
  - `spawn_exec_or_resume_flow(...)` uses the builder's existing `add_dirs(...)` surface before
    `build()`.
  - Empty lists emit no `--add-dir`; non-empty lists preserve normalized first-occurrence order.
  - No resume-only branch silently drops accepted directories.
- **Dependencies**:
  - `S1.T2` for `policy.add_dirs`
  - AD-C04 and AD-C07 from SEAM-1
  - Existing builder support in `crates/codex/src/builder/mod.rs` as an input surface
- **Verification**:
  - `cargo test -p agent_api codex`
  - Inspect the exec/resume path to confirm the builder, not bespoke argv code, owns
    `--add-dir` emission.
- **Rollout/safety**:
  - Safe default remains unchanged when the key is absent because the builder receives an empty
    list and emits no new argv.

#### S2.T1 — Thread the normalized add-dir list into `ExecFlowRequest` and the Codex builder

- **Outcome**: the exec/resume spawn path receives `policy.add_dirs` and hands it to the wrapper
  builder using the existing `add_dirs(...)` surface.
- **Files**:
  - `crates/agent_api/src/backends/codex/harness.rs`
  - `crates/agent_api/src/backends/codex/exec.rs`
  - Evidence-only input: `crates/codex/src/builder/mod.rs`

Checklist:
- Implement:
  - Add `add_dirs: Vec<PathBuf>` to `ExecFlowRequest`.
  - Populate that field in `CodexHarnessAdapter::spawn(...)`.
  - Call `builder.add_dirs(add_dirs)` in `spawn_exec_or_resume_flow(...)` before `build()`.
  - Preserve existing builder ordering so model selection remains earlier in argv than add-dir
    emission.
- Test:
  - Run `cargo test -p agent_api codex`.
- Validate:
  - Confirm fresh exec and resume use the same typed add-dir input.
  - Confirm the exec path delegates emission ownership to the builder.
  - Confirm empty lists remain omission-only behavior.
