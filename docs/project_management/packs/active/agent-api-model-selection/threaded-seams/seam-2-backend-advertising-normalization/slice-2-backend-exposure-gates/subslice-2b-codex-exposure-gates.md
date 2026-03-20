### S2b — Codex exposure gate and adapter guard tests

- **User/system value**: Keeps Codex model-key exposure truthful by tying admission and public advertising to the exact deterministic support posture owned downstream by Codex flow mapping.
- **Scope (in/out)**:
  - In:
    - carry former `S2.T2`
    - wire Codex to the shared exposure decision defined in `S2a`
    - add or update Codex capability tests that pin the truthful exposure posture for the branch under test
    - add focused adapter-level coverage, when exposure is enabled, proving the typed S1 handoff is consumed without a Codex-local raw parser
  - Out:
    - Codex argv emission and runtime behavior changes owned by SEAM-3
    - any Claude Code backend changes
    - capability-matrix regeneration
- **Acceptance criteria**:
  - Codex exposure remains absent from both `supported_extension_keys()` and `capabilities()` until `MS-C06` deterministic outcomes are present.
  - When exposure is enabled, Codex capability tests pin the same truthful posture in both surfaces.
  - Any adapter-level test exercises only the typed `NormalizedRequest` handoff from S1 and does not introduce raw extension parsing in Codex modules.
- **Dependencies**:
  - `S2a`
  - S1 / `MS-C09`
  - `MS-C05`
  - `MS-C06`
- **Verification**:
  - `cargo test -p agent_api --features codex`
  - focused review that no Codex module parses `request.extensions["agent_api.config.model.v1"]`
- **Rollout/safety**:
  - do not flip Codex exposure early
  - if rollback is needed, remove the capability id and allowlist admission together

#### S2b.T1 — Codex truthful exposure posture and guard coverage

- **Outcome**: Codex exposes `agent_api.config.model.v1` only when its exec, resume, and fork posture is truthful, and Codex-specific tests pin that rule.
- **Files**:
  - `crates/agent_api/src/backends/codex/backend.rs`
  - `crates/agent_api/src/backends/codex/policy.rs`
  - `crates/agent_api/src/backends/codex/harness.rs`
  - `crates/agent_api/src/backends/codex/tests/capabilities.rs`
  - optionally `crates/agent_api/src/backends/codex/tests/` for a focused adapter guard module

Checklist:
- Implement:
  - wire Codex exposure through the shared decision introduced in `S2a`
  - keep exposure absent until the deterministic mapping stack from `MS-C06` is present
  - when exposure becomes truthful, add a narrow adapter-level test that reaches the typed handoff from S1
- Test:
  - run `cargo test -p agent_api --features codex`
- Validate:
  - confirm any enabled exposure reaches the S1 typed handoff instead of `UnsupportedCapability`
  - confirm no Codex-local raw parse of the model key appears in backend, policy, or harness modules
