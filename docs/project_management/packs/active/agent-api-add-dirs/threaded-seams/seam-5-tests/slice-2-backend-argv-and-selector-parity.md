### S2 — Backend argv + selector-branch parity

- Status: decomposed for single-session execution because the original slice spans both backend
  families, capability publication, argv placement assertions, and the Codex fork rejection
  boundary across separate test modules and fake fixtures.
- Archived original: `archive/slice-2-backend-argv-and-selector-parity.md`
- Sub-slice directory: `slice-2-backend-argv-and-selector-parity/`

#### Why this was split

- `audit_slices.py` marks the file `OK` by simple line/task counts, but the real touch surface is
  larger than one Codex session: Codex exec/resume argv parity, Codex fork rejection boundaries,
  and Claude selector-branch placement each rely on different backend-local tests and fake
  binaries.
- Completing the original slice in one pass would patch roughly six or more distinct files/modules
  across `crates/agent_api/src/backends/codex/**`, `crates/agent_api/src/backends/claude_code/**`,
  and two separate fake CLI/app-server fixtures.
- The safest seam split is by backend concern: keep simple Codex argv parity separate from Codex
  fork rejection behavior, and keep Claude argv/selector parity in its own backend-local pass.

#### Sub-slices

- `S2a` → `slice-2-backend-argv-and-selector-parity/subslice-1-codex-capability-and-exec-resume-parity.md`
  - Covers original `S2.T1`: Codex capability publication plus exec/resume repeated-pair argv
    ordering, normalized order preservation, and absence semantics.
- `S2b` → `slice-2-backend-argv-and-selector-parity/subslice-2-codex-fork-rejection-boundary.md`
  - Covers original `S2.T2`: Codex fork selector `"last"` / `"id"` accepted-input rejection,
    invalid-input precedence, and zero-request app-server boundaries.
- `S2c` → `slice-2-backend-argv-and-selector-parity/subslice-3-claude-capability-and-selector-placement.md`
  - Covers original `S2.T3`: Claude capability publication plus fresh/resume/fork `--add-dir`
    group placement for selector-specific argv branches.
