### S2 — Backend exposure gates + no-second-parser adoption

- **Status**: Decomposed into sub-slices sized for a single Codex session.
- **Why decomposed**:
  - mixes one cross-backend exposure-coupling concern with two backend-specific rollout and test tracks
  - spans both Codex and Claude Code backend surfaces
  - would likely touch more than six distinct files once capability and adapter guards are included
- **Sub-slice directory**: `slice-2-backend-exposure-gates/`
- **Archived original**: `archive/slice-2-backend-exposure-gates.md`

#### Sub-slices

- `slice-2-backend-exposure-gates/subslice-2a-shared-exposure-coupling.md`
  - Carries former `S2.T1`: define one authoritative exposure decision per backend and keep R0 admission coupled to public advertising.
- `slice-2-backend-exposure-gates/subslice-2b-codex-exposure-gates.md`
  - Carries former `S2.T2`: Codex-specific exposure posture, capability tests, and typed-handoff adapter guard coverage.
- `slice-2-backend-exposure-gates/subslice-2c-claude-code-exposure-gates.md`
  - Carries former `S2.T3`: Claude Code-specific exposure posture, capability tests, and typed-handoff adapter guard coverage.
