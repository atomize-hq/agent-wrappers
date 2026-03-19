# S2 — Backend exposure gates + no-second-parser adoption

- **User/system value**: Couples R0 admission and public advertising for `agent_api.config.model.v1`, so built-in backends either expose a truthful deterministic-support posture or fail closed without drift between `supported_extension_keys()` and `capabilities()`.
- **Scope (in/out)**:
  - In:
    - wire the shared model-selection constant into backend allowlists and capability sets
    - keep one authoritative exposure decision per backend for the model key
    - add backend capability and adapter tests that pin allowlist/advertising coupling
  - Out:
    - actual `--model <trimmed-id>` argv emission for Codex and Claude Code
    - fork/runtime rejection translation and stream error behavior
    - capability-matrix regeneration (S3)
- **Acceptance criteria**:
  - Each built-in backend has one authoritative model-selection exposure predicate or equivalent local rule that drives both `supported_extension_keys()` and `capabilities()`.
  - The model key is not admitted at R0 or advertised publicly on a built-in backend until that backend has the deterministic flow outcomes required by `MS-C05`.
  - Once exposure is enabled, `normalize_request(...)` accepts the key for that backend and downstream code still consumes only the typed `NormalizedRequest` field from S1.
  - Backend capability tests fail if allowlist posture and advertising posture diverge.
- **Dependencies**:
  - S1 / `MS-C09`
  - `MS-C05`
  - final truthful exposure for Codex depends on the deterministic flow outcomes owned by `MS-C06`
  - final truthful exposure for Claude Code depends on the deterministic flow outcomes owned by `MS-C07`
- **Verification**:
  - `cargo test -p agent_api --features codex`
  - `cargo test -p agent_api --features claude_code`
  - focused review that no backend policy or harness module adds a second raw parser for the model key
- **Rollout/safety**:
  - Keep exposure false until the same change stack includes the downstream mapping support that makes it truthful.
  - Never land `supported_extension_keys()` and `capabilities()` changes for this key in separate changes.

## Atomic Tasks

#### S2.T1 — Couple model-key allowlists and public capability advertising with one backend-local decision per backend

- **Outcome**: Codex and Claude backend surfaces derive model-key admission and advertising from one local decision each, so R0 and `capabilities()` cannot drift.
- **Inputs/outputs**:
  - Input:
    - `crates/agent_api/src/backends/codex/policy.rs`
    - `crates/agent_api/src/backends/codex/backend.rs`
    - `crates/agent_api/src/backends/claude_code/mod.rs`
    - `crates/agent_api/src/backends/claude_code/backend.rs`
  - Output:
    - `crates/agent_api/src/backends/codex/policy.rs`
    - `crates/agent_api/src/backends/codex/backend.rs`
    - `crates/agent_api/src/backends/claude_code/mod.rs`
    - `crates/agent_api/src/backends/claude_code/backend.rs`
- **Implementation notes**:
  - Reuse the shared constant from S1 instead of introducing another string literal.
  - Prefer a small helper or tightly-scoped local block per backend so both of these are driven by the same truth:
    - `supported_extension_keys()`
    - `capabilities()`
  - Do not split admission from advertising:
    - if the backend is not yet deterministic for all exposed flows, keep both surfaces false
    - once the deterministic mapping change lands, flip both surfaces in the same stack
  - Remember that `AgentWrapperCapabilities.ids` is backend-global, not per-flow; partial flow support is not enough.
- **Acceptance criteria**:
  - A backend cannot advertise `agent_api.config.model.v1` while still rejecting the same key as unsupported at R0.
  - A backend cannot admit the key at R0 while leaving public advertising false for the same final deterministic-support posture.
- **Test notes**:
  - Covered by S2.T2 and S2.T3.
- **Risk/rollback notes**:
  - Medium: a mismatched flip would violate the public contract, so keep the change tightly scoped.

Checklist:
- Implement: add one authoritative exposure decision per backend and reuse it in both admission and advertising code paths.
- Test: backend capability tests prove admission and advertising match.
- Validate: review `supported_extension_keys()` and `capabilities()` diffs together.
- Cleanup: avoid helper sprawl; keep the decision close to the backend surface.

#### S2.T2 — Wire Codex model-key exposure and adapter-level guard tests

- **Outcome**: The Codex built-in backend exposes `agent_api.config.model.v1` only when its exec/resume/fork flow set satisfies the pinned deterministic outcomes, and its tests pin that posture.
- **Inputs/outputs**:
  - Input:
    - `crates/agent_api/src/backends/codex/backend.rs`
    - `crates/agent_api/src/backends/codex/policy.rs`
    - `crates/agent_api/src/backends/codex/harness.rs`
    - `crates/agent_api/src/backends/codex/tests/capabilities.rs`
  - Output:
    - `crates/agent_api/src/backends/codex/backend.rs`
    - `crates/agent_api/src/backends/codex/policy.rs`
    - `crates/agent_api/src/backends/codex/tests/capabilities.rs`
    - optionally a focused Codex model-selection adapter test module under `crates/agent_api/src/backends/codex/tests/`
- **Implementation notes**:
  - Codex exposure is truthful only when:
    - exec and resume map the typed trimmed model id to exactly one `--model <trimmed-id>` pair, and
    - fork keeps the pinned pre-handle safe rejection path
  - Until that mapping stack is present, keep the Codex model capability absent from both `supported_extension_keys()` and `capabilities()`.
  - When exposure flips true, add an adapter-level normalization test that proves a valid model key is admitted past R0 without any Codex-local raw parse of the key.
- **Acceptance criteria**:
  - Codex capability tests pin the truthful posture for the branch under test.
  - Any adapter-level test touches only the typed handoff from S1 and does not add raw `request.extensions["agent_api.config.model.v1"]` parsing in Codex modules.
- **Test notes**:
  - Run: `cargo test -p agent_api --features codex`.
- **Risk/rollback notes**:
  - Medium: do not flip Codex exposure early; rollback is removing the capability id and allowlist entry together.

Checklist:
- Implement: wire Codex exposure through the shared decision from S2.T1.
- Test: `cargo test -p agent_api --features codex`.
- Validate: if exposure is enabled, confirm an adapter-level test reaches the S1 typed handoff instead of `UnsupportedCapability`.
- Cleanup: keep Codex-specific changes out of argv-mapping modules owned by SEAM-3.

#### S2.T3 — Wire Claude Code model-key exposure and adapter-level guard tests

- **Outcome**: The Claude Code built-in backend exposes `agent_api.config.model.v1` only when its print exec/resume/fork flow set satisfies the pinned deterministic outcomes, and its tests pin that posture.
- **Inputs/outputs**:
  - Input:
    - `crates/agent_api/src/backends/claude_code/backend.rs`
    - `crates/agent_api/src/backends/claude_code/mod.rs`
    - `crates/agent_api/src/backends/claude_code/harness.rs`
    - `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`
  - Output:
    - `crates/agent_api/src/backends/claude_code/backend.rs`
    - `crates/agent_api/src/backends/claude_code/mod.rs`
    - `crates/agent_api/src/backends/claude_code/tests/capabilities.rs`
    - optionally a focused Claude model-selection adapter test module under `crates/agent_api/src/backends/claude_code/tests/`
- **Implementation notes**:
  - Claude exposure is truthful only when print exec/resume/fork all emit exactly one `--model <trimmed-id>` pair and the key never aliases to `--fallback-model`.
  - Until that mapping stack is present, keep the Claude model capability absent from both `supported_extension_keys()` and `capabilities()`.
  - When exposure flips true, add an adapter-level normalization test that proves a valid model key is admitted past R0 without any Claude-local raw parse of the key.
- **Acceptance criteria**:
  - Claude capability tests pin the truthful posture for the branch under test.
  - Adapter-level tests confirm the typed handoff path without introducing a second parser in Claude modules.
- **Test notes**:
  - Run: `cargo test -p agent_api --features claude_code`.
- **Risk/rollback notes**:
  - Medium: do not flip Claude exposure early; rollback is removing the capability id and allowlist entry together.

Checklist:
- Implement: wire Claude exposure through the shared decision from S2.T1.
- Test: `cargo test -p agent_api --features claude_code`.
- Validate: if exposure is enabled, confirm the adapter test reaches the S1 typed handoff and never maps to `--fallback-model`.
- Cleanup: keep Claude-specific changes out of argv-mapping modules owned by SEAM-4.
