### S2c — Claude Code exposure gate and adapter guard tests

- **User/system value**: Keeps Claude Code model-key exposure truthful by tying admission and public advertising to the exact deterministic support posture owned downstream by Claude print exec, resume, and fork mapping.
- **Scope (in/out)**:
  - In:
    - carry former `S2.T3`
    - wire Claude Code to the shared exposure decision defined in `S2a`
    - add or update Claude Code capability tests that pin the truthful exposure posture for the branch under test
    - add focused adapter-level coverage, when exposure is enabled, proving the typed S1 handoff is consumed without a Claude-local raw parser or fallback-model aliasing
  - Out:
    - Claude argv emission and runtime behavior changes owned by SEAM-4
    - any Codex backend changes
    - capability-matrix regeneration
- **Acceptance criteria**:
  - Claude Code exposure remains absent from both `supported_extension_keys()` and `capabilities()` until `MS-C07` deterministic outcomes are present.
  - When exposure is enabled, Claude Code capability tests pin the same truthful posture in both surfaces.
  - Any adapter-level test exercises only the typed `NormalizedRequest` handoff from S1 and confirms the key never aliases to `--fallback-model`.
- **Dependencies**:
  - `S2a`
  - S1 / `MS-C09`
  - `MS-C05`
  - `MS-C07`
- **Verification**:
  - `cargo test -p agent_api --features claude_code`
  - focused review that no Claude Code module parses `request.extensions["agent_api.config.model.v1"]`
- **Rollout/safety**:
  - do not flip Claude Code exposure early
  - if rollback is needed, remove the capability id and allowlist admission together

#### S2c.T1 — Claude Code truthful exposure posture and guard coverage

- **Outcome**: Claude Code exposes `agent_api.config.model.v1` only when its print exec, resume, and fork posture is truthful, and Claude-specific tests pin that rule.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/backend.rs`
  - `crates/agent_api/src/backends/claude_code/mod.rs`
  - `crates/agent_api/src/backends/claude_code/harness.rs`
  - `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`
  - optionally `crates/agent_api/src/backends/claude_code/tests/` for a focused adapter guard module

Checklist:
- Implement:
  - wire Claude Code exposure through the shared decision introduced in `S2a`
  - keep exposure absent until the deterministic mapping stack from `MS-C07` is present
  - when exposure becomes truthful, add a narrow adapter-level test that reaches the typed handoff from S1 and never aliases to `--fallback-model`
- Test:
  - run `cargo test -p agent_api --features claude_code`
- Validate:
  - confirm any enabled exposure reaches the S1 typed handoff rather than a Claude-local raw parser
  - confirm the model key never maps to `--fallback-model`
