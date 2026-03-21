# SEAM-4 — Claude Code backend mapping (threaded decomposition)

> Pack: `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/`
> Seam brief: `seam-4-claude-code-mapping.md`
> Threading source of truth: `threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-4
- **Name**: Claude Code mapping for `agent_api.exec.external_sandbox.v1`
- **Goal / value**: when enabled + requested, run Claude Code in a mode compatible with external
  sandboxing by relaxing internal permission guardrails without prompting.
- **Type**: capability (backend mapping) + integration (CLI version differences)
- **Scope**
  - In:
    - Validate `extensions["agent_api.exec.external_sandbox.v1"]` as boolean (default `false`)
      before spawn (when the capability is enabled).
    - Enforce contradiction rules (pre-spawn; fail-closed):
      - `external_sandbox=true` + `agent_api.exec.non_interactive=false` →
        `AgentWrapperError::InvalidRequest` (ES-C02)
      - `external_sandbox=true` + any `backend.claude_code.exec.*` key present →
        `AgentWrapperError::InvalidRequest` (ES-C06)
    - Emit the pinned warning `Status` event when `external_sandbox=true` is accepted (exact
      message + ordering per `docs/specs/universal-agent-api/extensions-spec.md`).
    - Apply the pinned mapping contract (ES-C05) across all Claude Code `claude --print` flows:
      - include `--dangerously-skip-permissions` when `external_sandbox=true`.
    - Ensure required allow-flag behavior is deterministic pre-spawn (ES-C07):
      - determine allow-flag support via a cached `claude --help` preflight (no spawn+retry loop),
      - include `--allow-dangerously-skip-permissions` **iff** supported, and
      - fail before spawn as `AgentWrapperError::Backend { .. }` when preflight cannot be
        performed deterministically and the key is requested.
  - Out:
    - Host opt-in + capability advertising posture (`allow_external_sandbox_exec`) (SEAM-2).
    - Core key semantics / R0 precedence / warning event contract text (SEAM-1).
    - Regression test coverage for the final behavior (SEAM-5).
- **Touch surface**:
  - `crates/agent_api/src/backends/claude_code.rs`
- **Verification**:
  - Local compile + existing Claude backend tests: `cargo test -p agent_api claude_code`
  - SEAM-5 adds pinned mapping/validation tests once this code lands.
- **Threading constraints**
  - Upstream blockers: SEAM-1 (ES-C01/02/06) + SEAM-2 (ES-C03)
  - Downstream blocked seams: SEAM-5
  - Contracts produced (owned): ES-C05, ES-C07
  - Contracts consumed: ES-C01, ES-C02, ES-C03, ES-C06

Implementation note: `docs/specs/claude-code-session-mapping-contract.md` is the canonical,
Normative source of truth for ES-C05/ES-C07. Treat the slice below as a code conformance checklist.

## Slice index

- `S1` → `slice-1-external-sandbox-mapping.md`: validate + warn + deterministically map external
  sandbox mode for Claude Code `claude --print`, including the cached `--help` allow-flag preflight.

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `ES-C05`: Claude mapping contract — when enabled + requested, Claude Code maps external sandbox
    mode to `claude --print --dangerously-skip-permissions ...` and applies any additional required
    opt-in allow flag deterministically (canonical:
    `docs/specs/claude-code-session-mapping-contract.md`).
    - Produced by: `S1` (argv mapping + deterministic behavior).
  - `ES-C07`: Claude allow-flag preflight (external sandbox mode) — allow-flag support is
    determined pre-spawn via a deterministic cached `claude --help` preflight and MUST NOT use a
    spawn+retry loop (canonical: `docs/specs/claude-code-session-mapping-contract.md`).
    - Produced by: `S1.T3` (help preflight + caching + fail-before-spawn behavior).
- **Contracts consumed**:
  - `ES-C01`: External sandbox execution policy extension key — the key id
    `agent_api.exec.external_sandbox.v1` (boolean; validated before spawn) owned by SEAM-1
    (`docs/specs/universal-agent-api/extensions-spec.md`).
    - Consumed by: `S1.T1` (value validation + extraction) and `S1.T2` (warning gating).
  - `ES-C02`: Non-interactive invariant — `external_sandbox=true` MUST NOT be combined with
    `agent_api.exec.non_interactive=false` (InvalidRequest) owned by SEAM-1.
    - Consumed by: `S1.T1` (contradiction validation).
  - `ES-C03`: Safe default advertising — built-in backends MUST NOT advertise the key by default;
    externally sandboxed hosts opt-in explicitly via backend configuration
    `allow_external_sandbox_exec` (canonical: `docs/specs/universal-agent-api/contract.md`).
    - Consumed by: `S1` (assumption: mapping runs only when opt-in is enabled).
  - `ES-C06`: Exec-policy combination rule — forbid any `backend.<agent_kind>.exec.*` keys when
    `external_sandbox=true` (InvalidRequest) owned by SEAM-1.
    - Consumed by: `S1.T1` (reject `backend.claude_code.exec.*` keys when `external_sandbox=true`).
- **Dependency edges honored**:
  - `SEAM-2 blocks SEAM-4`: this plan assumes Claude Code only supports the key behind explicit
    host opt-in (ES-C03); mapping is unreachable until SEAM-2 exposes the capability id and
    allowlists the key under R0.
  - `SEAM-4 blocks SEAM-5`: `S1` provides the final Claude mapping behavior that SEAM-5 pins in
    tests.
- **Parallelization notes**:
  - What can proceed now: prepare SEAM-4 mapping implementation details behind a feature branch;
    keep changes tightly scoped to avoid conflict with SEAM-2’s enablement edits in
    `crates/agent_api/src/backends/claude_code.rs`.
  - What must wait: `S1` requires SEAM-2’s opt-in enablement to make the key supported; SEAM-5 must
    wait for `S1` to finalize mapping behavior.

