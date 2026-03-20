# S2 — Isolated homes (backend config + wiring)

- **User/system value**: Enables safe automation and deterministic tests by confining MCP state/config mutations to an
  explicit, backend-scoped home directory (no user-state mutation unless the host opts in).
- **Scope (in/out)**:
  - In:
    - Ensure built-in backends support isolated homes via host-provided config:
      - Codex: `CodexBackendConfig.codex_home: Option<PathBuf>` (already present; keep behavior pinned)
      - Claude: `ClaudeCodeBackendConfig.claude_home: Option<PathBuf>` (add + wire)
    - Ensure env precedence is pinned: request-level env overrides win over any isolated-home env injection (so a caller
      can intentionally defeat isolation if they override `HOME`/`XDG_*`).
  - Out:
    - Cross-backend hermetic fake-binary harnesses and integration tests (SEAM-5).
    - Backend mapping of MCP commands to argv (SEAM-3/4), except where home injection must be plumbed into the wrapper builder.
- **Acceptance criteria**:
  - Claude backend config includes `claude_home: Option<PathBuf>` and the backend uses it to inject the correct env layout
    for subprocesses (via wrapper builder), without mutating the parent env.
  - When `claude_home` is `Some`, the backend still honors request env overrides (request keys win).
  - Codex behavior remains pinned: when `codex_home` is `Some`, subprocesses operate under the isolated root.
- **Dependencies**:
  - MM-C07 contract definition (owned here) is pinned in `docs/specs/universal-agent-api/mcp-management-spec.md`.
  - MM-C03 precedence rules (owned by SEAM-1) for request env overrides.
- **Verification**:
  - `cargo test -p agent_api --features claude_code`
  - (If wiring touches wrapper crates) targeted wrapper tests:
    - `cargo test -p claude_code`
    - `cargo test -p codex`
- **Rollout/safety**:
  - Defaults remain unchanged (`*_home: None`) unless the host opts in.
  - Isolation is best-effort; explicit env overrides remain authoritative.

## Atomic Tasks

#### S2.T1 — Add `claude_home` to `ClaudeCodeBackendConfig` (host-provided) and plumb into the Claude client builder

- **Outcome**: The Claude built-in backend can run under an isolated, backend-scoped home directory.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Safety posture” → “isolated homes MUST be supported”)
  - Output:
    - `crates/agent_api/src/backends/claude_code.rs`: add `claude_home: Option<PathBuf>` to config
    - `crates/agent_api/src/backends/claude_code.rs`: when building `claude_code::ClaudeClient`, call
      `ClaudeClientBuilder::claude_home(...)` when configured
- **Implementation notes**:
  - Apply the home override before applying request env overrides so request keys can intentionally override (pinned).
  - Do not mutate the parent env; rely on wrapper builder env injection only.
- **Acceptance criteria**:
  - With `claude_home: Some(path)`, the built client is configured for isolated-home env injection.
  - With request env overrides, request keys win (e.g., explicitly setting `HOME` overrides the injected value).
- **Test notes**:
  - Prefer unit tests that validate builder configuration without spawning the real `claude` binary (see S2.T3).
- **Risk/rollback notes**: additive config + wiring; safe.

Checklist:
- Implement: add config field + builder wiring.
- Test: `cargo check -p agent_api --features claude_code`.
- Validate: ensure request env overrides are applied after home injection.
- Cleanup: rustfmt.

#### S2.T2 — Confirm Codex isolated-home wiring remains pinned (no regression)

- **Outcome**: Codex continues to respect `codex_home` for subprocesses and preserves request env precedence rules.
- **Inputs/outputs**:
  - Input: existing `CodexBackendConfig.codex_home` usage in `crates/agent_api/src/backends/codex/*`
  - Output: (optional) minimal refactor or tests, only if needed to make behavior explicit + reusable by SEAM-3 mapping.
- **Implementation notes**:
  - Avoid touching SEAM-3/4 mapping surfaces; keep this strictly about config plumbing and precedence.
  - If factoring helpers for reuse, keep them private and narrowly scoped (e.g., “apply codex_home to builder”).
- **Acceptance criteria**:
  - Codex backend still injects `CODEX_HOME` (via wrapper) when configured and does not mutate parent env.
- **Test notes**:
  - Prefer existing wrapper tests in `crates/codex` when possible; add agent_api tests only if behavior is not already pinned.
- **Risk/rollback notes**: low; avoid unnecessary changes.

Checklist:
- Implement: only if required (prefer no-op confirmation).
- Test: run targeted tests if touched.
- Validate: no behavior drift for run flows.
- Cleanup: keep diffs minimal.

#### S2.T3 — Add unit tests pinning isolated-home precedence for Claude (request env overrides win)

- **Outcome**: Deterministic tests that enforce the pinned precedence rule: request env overrides win over isolated-home env injection.
- **Inputs/outputs**:
  - Input: MM-C03 precedence rules + `claude_code::ClaudeClientBuilder` behavior
  - Output: unit tests under `crates/agent_api` (preferred) or `crates/claude_code` (if agent_api cannot observe builder state cleanly)
- **Implementation notes**:
  - If agent_api wiring is hard to observe, factor out a small helper that builds a `ClaudeClientBuilder` given config +
    request env, and unit test that helper without spawning.
  - Assert at minimum:
    - `claude_home: Some(x)` and no request override → builder env includes injected `HOME`/`XDG_*` consistent with x,
    - request override for `HOME` (or `XDG_CONFIG_HOME`) wins over injected defaults.
- **Acceptance criteria**:
  - Tests fail if injection order regresses or if request env no longer wins.
- **Test notes**:
  - Run: `cargo test -p agent_api --features claude_code`.
- **Risk/rollback notes**: tests-only; safe.

Checklist:
- Implement: unit tests (and minimal helper if needed).
- Test: `cargo test -p agent_api --features claude_code`.
- Validate: tests do not spawn real `claude`.
- Cleanup: keep tests narrowly focused on precedence.

## Notes for downstream seams (non-tasking)

- SEAM-3/4 MCP hook implementations MUST apply the configured isolated home when spawning management commands, and MUST
  still honor request env overrides (request keys win).
