### S2 — Exec/resume repeated `--add-dir` mapping

- **User/system value**: makes accepted add-dir inputs reach Codex exec and resume flows with a
  deterministic repeated-flag argv layout that downstream tests can pin exactly.
- **Scope (in/out)**:
  - In:
    - Thread `policy.add_dirs` into `ExecFlowRequest`.
    - Use the existing Codex wrapper builder support to emit one repeated `--add-dir <DIR>` pair
      per normalized unique directory on fresh exec and resume flows.
    - Preserve order and absence semantics.
    - Keep any accepted `--model <trimmed-id>` pair earlier in argv than the first emitted
      `--add-dir`.
    - Keep `docs/specs/codex-streaming-exec-contract.md` aligned with the implemented mapping.
  - Out:
    - Capability advertising and shared normalization (S1).
    - Fork rejection behavior (S3).
    - Pack-level regression coverage and capability-matrix regeneration (SEAM-5).
- **Acceptance criteria**:
  - When `policy.add_dirs` is empty, Codex exec/resume emits no `--add-dir`.
  - When `policy.add_dirs` is present, exec/resume emits one repeated `--add-dir <DIR>` pair per
    directory, preserving first-occurrence order from the normalized list.
  - Resume uses the same effective add-dir list that a fresh exec run would emit.
  - Any accepted `--model <trimmed-id>` pair appears before the first `--add-dir`.
  - Downstream code uses the typed list from `ExecFlowRequest`, not the raw extension payload.
- **Dependencies**:
  - `S1.T2` for `policy.add_dirs`
  - AD-C05 in `docs/specs/codex-streaming-exec-contract.md`
  - AD-C04/AD-C07 from SEAM-1
- **Verification**:
  - `cargo test -p agent_api codex`
  - End-to-end argv assertions are owned by SEAM-5 after this slice lands.
- **Rollout/safety**:
  - Safe default when the key is absent: the builder receives an empty list and emits no new argv.

#### S2.T1 — Thread the normalized add-dir list into `ExecFlowRequest` and the Codex builder

- **Outcome**: the exec/resume spawn path receives `policy.add_dirs` and hands it to the wrapper
  builder using the existing `add_dirs(...)` surface.
- **Inputs/outputs**:
  - Input: `policy.add_dirs` from `S1.T2`; existing wrapper support in
    `crates/codex/src/builder/mod.rs`.
  - Output:
    - `crates/agent_api/src/backends/codex/harness.rs`
    - `crates/agent_api/src/backends/codex/exec.rs`
- **Implementation notes**:
  - Extend `ExecFlowRequest` with `add_dirs: Vec<PathBuf>`.
  - Populate that field in `CodexHarnessAdapter::spawn(...)`.
  - In `spawn_exec_or_resume_flow(...)`, call `builder.add_dirs(add_dirs)` before `build()`.
  - Preserve the existing builder ordering so model selection remains earlier in argv than
    add-dir emission.
- **Acceptance criteria**:
  - Fresh exec and resume both use the same typed add-dir input.
  - Empty lists do not emit `--add-dir`.
  - Non-empty lists preserve normalized order exactly.
- **Test notes**:
  - Run `cargo test -p agent_api codex`.
  - Defer detailed argv snapshots and selector-branch assertions to SEAM-5.
- **Risk/rollback notes**:
  - Low-to-moderate risk because the wrapper builder already owns `--add-dir` emission; this task
    is primarily plumbing and ordering preservation.

Checklist:
- Implement: add `add_dirs: Vec<PathBuf>` to `ExecFlowRequest` and pass it into the Codex builder.
- Test: `cargo test -p agent_api codex`.
- Validate: inspect the exec path to ensure the builder, not bespoke argv code, owns `--add-dir`
  emission.
- Cleanup: avoid any resume-only special case that could silently drop accepted directories.

#### S2.T2 — Pin the exec/resume mapping truth in the Codex streaming contract doc

- **Outcome**: the backend-owned contract doc names the exact add-dir argv behavior that the code
  implements, so SEAM-5 can pin against one stable source of truth.
- **Inputs/outputs**:
  - Input: AD-C05 from `threading.md`; the live Codex mapping behavior implemented in `S2.T1`.
  - Output:
    - `docs/specs/codex-streaming-exec-contract.md`
- **Implementation notes**:
  - Keep the document explicit about:
    - repeated `--add-dir <DIR>` pairs,
    - order preservation,
    - omission when absent,
    - model-before-add-dir placement,
    - resume parity.
  - If implementation details change during landing, update the doc in the same change rather than
    leaving drift for SEAM-5 to discover.
- **Acceptance criteria**:
  - The streaming contract doc matches the implemented exec/resume mapping exactly.
  - There is no remaining ambiguity about placement or absence semantics.
- **Test notes**:
  - No standalone doc-only test; validation is that SEAM-5 can pin against this text without
    inventing new assumptions.
- **Risk/rollback notes**:
  - Low risk; the main failure mode is doc drift if this task is skipped.

Checklist:
- Implement: update `docs/specs/codex-streaming-exec-contract.md` in lockstep with the exec/resume
  mapping code.
- Test: `cargo test -p agent_api codex`.
- Validate: confirm the doc states repeated pairs, order preservation, absence semantics, and
  model-before-add-dir placement.
- Cleanup: remove any stale wording that suggests add-dir support is unimplemented or undefined.
