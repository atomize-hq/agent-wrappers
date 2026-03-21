### S2b — Streaming contract alignment for repeated `--add-dir` mapping

- **User/system value**: gives SEAM-5 and future maintainers one backend-owned, Normative source
  of truth for how Codex exec/resume emits accepted add-dir inputs.
- **Scope (in/out)**:
  - In:
    - Update the Codex streaming exec contract doc in lockstep with the runtime mapping from
      `S2a`.
    - State repeated `--add-dir <DIR>` pairs, order preservation, omission when absent,
      model-before-add-dir placement, and resume parity explicitly.
    - Remove stale wording that implies add-dir behavior is undefined or unimplemented.
  - Out:
    - Runtime plumbing in `crates/agent_api/...` (`S2a`).
    - Fork rejection behavior (`S3`).
    - Pack-level regression coverage and capability-matrix regeneration (`SEAM-5`).
- **Acceptance criteria**:
  - `docs/specs/codex-streaming-exec-contract.md` matches the implemented exec/resume mapping
    exactly.
  - The contract text is explicit about repeated pairs, order preservation, omission semantics,
    model-before-add-dir placement, and resume parity.
  - SEAM-5 can pin against the contract without inventing any additional assumptions.
- **Dependencies**:
  - AD-C05 in `docs/specs/codex-streaming-exec-contract.md`
  - Runtime truth implemented by `S2a`
- **Verification**:
  - Re-read `docs/specs/codex-streaming-exec-contract.md` after the update and compare it against
    the landed `S2a` behavior.
  - Run `cargo test -p agent_api codex` in the same change to keep code/doc drift visible.
- **Rollout/safety**:
  - This sub-slice should land in the same change as `S2a` or immediately after it; do not leave
    the Normative doc stale between sessions.

#### S2.T2 — Pin the exec/resume mapping truth in the Codex streaming contract doc

- **Outcome**: the backend-owned contract doc names the exact add-dir argv behavior that the code
  implements, so SEAM-5 can pin against one stable source of truth.
- **Files**:
  - `docs/specs/codex-streaming-exec-contract.md`

Checklist:
- Implement:
  - Update the streaming contract doc in lockstep with the exec/resume mapping code.
  - State repeated `--add-dir <DIR>` pairs, order preservation, omission when absent,
    model-before-add-dir placement, and resume parity explicitly.
  - Remove stale wording that suggests add-dir support is unimplemented or undefined.
- Test:
  - Run `cargo test -p agent_api codex`.
- Validate:
  - Confirm there is no remaining ambiguity about placement or absence semantics.
  - Confirm the written contract matches the `S2a` implementation rather than speculative future
    behavior.
