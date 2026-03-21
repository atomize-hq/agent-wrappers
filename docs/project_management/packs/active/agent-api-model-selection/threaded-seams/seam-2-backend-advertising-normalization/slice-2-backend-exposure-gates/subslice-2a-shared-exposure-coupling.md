### S2a — Shared exposure coupling rule across built-in backends

- **User/system value**: Establishes one backend-local truth source for model-key exposure so built-in backends cannot drift between R0 admission and public advertising.
- **Scope (in/out)**:
  - In:
    - carry former `S2.T1`
    - define one authoritative exposure decision per backend for `agent_api.config.model.v1`
    - wire that decision into both `supported_extension_keys()` and `capabilities()`
    - keep the shared constant from S1 as the only key identifier
  - Out:
    - Codex adapter guard tests and backend-specific rollout proof
    - Claude Code adapter guard tests and backend-specific rollout proof
    - capability-matrix regeneration
- **Acceptance criteria**:
  - Codex has one local decision that drives both R0 admission and public advertising for the model key.
  - Claude Code has one local decision that drives both R0 admission and public advertising for the model key.
  - Neither backend admits the key at R0 while leaving public advertising false for the same deterministic-support posture.
  - Neither backend advertises the key publicly while still rejecting it at R0.
- **Dependencies**:
  - S1 / `MS-C09`
  - `MS-C05`
  - `MS-C06` readiness evidence for the eventual Codex exposure flip
  - `MS-C07` readiness evidence for the eventual Claude Code exposure flip
- **Verification**:
  - focused review of `supported_extension_keys()` and `capabilities()` diffs together for each backend
  - confirm no new string literal or second parser for `agent_api.config.model.v1`
- **Rollout/safety**:
  - keep exposure false on any backend whose deterministic mapping work is not yet landed
  - keep the exposure decision close to the backend surface to avoid helper sprawl

#### S2a.T1 — Backend-local exposure decisions drive both admission and advertising

- **Outcome**: Each built-in backend has one authoritative exposure rule for the model key, reused by both R0 allowlists and public capability advertising.
- **Files**:
  - `crates/agent_api/src/backends/codex/policy.rs`
  - `crates/agent_api/src/backends/codex/backend.rs`
  - `crates/agent_api/src/backends/claude_code/mod.rs`
  - `crates/agent_api/src/backends/claude_code/backend.rs`

Checklist:
- Implement:
  - add one authoritative exposure decision per backend
  - route both `supported_extension_keys()` and `capabilities()` through that same decision
  - reuse the shared model-selection constant from S1 rather than introducing another literal
- Test:
  - defer backend-specific capability assertions to `S2b` and `S2c`
- Validate:
  - review Codex and Claude Code surfaces together to confirm admission and advertising cannot drift
  - verify no backend module adds a second raw parse of `request.extensions["agent_api.config.model.v1"]`
