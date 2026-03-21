### S3c — Codex contract publication

- **User/system value**: publishes the final SEAM-3 behavior in the canonical Codex contracts so reviewers and downstream seams can rely on one normative description of mapping, fork rejection, and runtime failure posture.
- **Scope (in/out)**:
  - In:
    - update `docs/specs/codex-streaming-exec-contract.md` for exec/resume `--model` ordering and runtime failure wording
    - update `docs/specs/codex-app-server-jsonrpc-contract.md` for the fork pre-handle rejection posture and any app-server-visible implications
    - align the spec wording with the landed S1, S2, and S3a behavior plus the focused backend tests from S3b
  - Out:
    - implementation changes in runtime or backend code
    - new regression tests beyond any tiny doc-adjacent assertions needed for cross-reference accuracy
- **Acceptance criteria**:
  - both Codex spec docs match the final implementation without unresolved drift
  - exec/resume ordering around `--model` is explicitly documented
  - the fork pre-handle rejection posture is explicitly documented with the pinned safe backend message
  - SEAM-5B can cite these docs as the canonical SEAM-3 contract surface
- **Dependencies**:
  - landed `S1`, `S2`, and `S3a`
  - focused regression confirmation from `S3b`
- **Verification**:
  - spec diff review against the landed code and test paths
  - cross-check threading contract IDs and wording against `seam.md` / `threading.md`
- **Rollout/safety**:
  - keep the docs normative and concise
  - do not let pack-language or ADR wording override `docs/specs/**`

#### S3c.T1 — Publish the streaming exec contract updates

- **Outcome**: the streaming exec spec describes final exec/resume `--model` ordering and the safe runtime-rejection behavior reviewers should expect.
- **Files**:
  - `docs/specs/codex-streaming-exec-contract.md`

Checklist:
- Implement:
  - document the final ordering and runtime-failure posture
- Test:
  - compare the wording directly against landed code/tests rather than re-stating assumptions
- Validate:
  - confirm the contract matches the exact safe-message behavior and event/completion parity

#### S3c.T2 — Publish the app-server subset contract updates

- **Outcome**: the app-server contract documents the pre-handle fork rejection posture and keeps the unsupported-flow boundary explicit.
- **Files**:
  - `docs/specs/codex-app-server-jsonrpc-contract.md`

Checklist:
- Implement:
  - describe the fork rejection posture as part of the Codex app-server subset contract
- Test:
  - compare the wording against landed fork tests and backend behavior
- Validate:
  - confirm the contract does not imply unsupported app-server traffic occurs on the rejection path
