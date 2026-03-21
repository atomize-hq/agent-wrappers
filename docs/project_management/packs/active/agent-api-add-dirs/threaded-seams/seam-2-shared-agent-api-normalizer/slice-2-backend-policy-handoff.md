### S2 — Wire the shared helper into Codex and Claude policy extraction

- This slice was decomposed because it spans both built-in backends plus backend-local tests and Claude run-start cwd capture, which pushes it past the single-session budget for one Codex pass.
- Archived original: `archive/slice-2-backend-policy-handoff.md`
- Sub-slice directory: `slice-2-backend-policy-handoff/`

#### Audit Result

- `slice-2-backend-policy-handoff.md`: Oversized
- Reasons:
  - spans both Codex and Claude backend policy surfaces
  - requires changes across roughly six or more distinct files/modules (`policy.rs`, `harness.rs`, backend entrypoints, and backend-local tests)
  - bundles implementation and direct-policy verification for two backends into one slice

#### Decomposition Plan

- `S2a` covers Codex-only policy extraction and direct-policy tests.
- `S2b` covers Claude-only run-start cwd capture, policy extraction, and direct-policy tests.

#### Sub-slices

- `slice-2-backend-policy-handoff/subslice-1-codex-policy-handoff.md`
  - Carries the shared helper into `CodexExecPolicy`, resolves effective cwd inside policy extraction, and adds Codex-only direct-policy coverage.
- `slice-2-backend-policy-handoff/subslice-2-claude-policy-handoff.md`
  - Adds Claude run-start cwd capture, attaches normalized add-dirs to `ClaudeExecPolicy`, and adds Claude-only direct-policy coverage.
