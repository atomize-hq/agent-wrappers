# SEAM-2 — Backend enablement + capability advertising (threaded decomposition)

> Pack: `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/`
> Seam brief: `seam-2-backend-enablement.md`
> Threading source of truth: `threading.md`

## Seam Brief (Restated)

- **Seam ID**: SEAM-2
- **Name**: built-in backend opt-in for `agent_api.exec.external_sandbox.v1`
- **Goal / value**: ensure externally sandboxed hosts can opt-in to this dangerous capability, while
  built-in backends remain safe-by-default and do not advertise it automatically.
- **Type**: platform + risk
- **Scope**
  - In:
    - Add a host-controlled backend config toggle (default `false`) that gates both:
      - capability advertising in `capabilities().ids`, and
      - harness R0 allowlisting via `supported_extension_keys()`.
    - Apply to both built-in backends:
      - Codex: `crates/agent_api/src/backends/codex.rs`
      - Claude Code: `crates/agent_api/src/backends/claude_code.rs`
    - Keep the harness allowlist and capability ids aligned (no “advertise but reject”, or
      “accept but don’t advertise”).
    - Add unit tests that pin the default safe posture and opt-in behavior.
  - Out:
    - Core key semantics + contradiction rules (SEAM-1).
    - Backend-specific CLI mapping (SEAM-3, SEAM-4).
    - Cross-backend regression tests (SEAM-5).
- **Touch surface**:
  - `crates/agent_api/src/backends/codex.rs`
  - `crates/agent_api/src/backends/claude_code.rs`
  - `crates/agent_api/src/backends/codex/tests.rs`
  - `crates/agent_api/src/backends/claude_code/tests.rs`
- **Verification**:
  - Unit tests pin:
    - default instances do not advertise `agent_api.exec.external_sandbox.v1`,
    - opt-in instances do advertise it, and
    - R0 gating fails closed as `UnsupportedCapability` when opt-in is disabled.
- **Threading constraints**
  - Upstream blockers: SEAM-1 (ES-C01 semantics + validation ordering)
  - Downstream blocked seams: SEAM-3, SEAM-4 (direct), SEAM-5 (transitive)
  - Contracts produced (owned): ES-C03
  - Contracts consumed: ES-C01

Implementation note: `docs/specs/universal-agent-api/contract.md` already contains a normative
“Dangerous capability opt-in (external sandbox exec policy)” section (including the
`allow_external_sandbox_exec` field and required behaviors). Treat the slices below as a code
conformance checklist unless the contract doc needs edits.

## Slice index

- `S1` → `slice-1-codex-opt-in-gating.md`: gate Codex advertising + R0 allowlist behind explicit
  host opt-in.
- `S2` → `slice-2-claude-opt-in-gating.md`: gate Claude Code advertising + R0 allowlist behind
  explicit host opt-in.

## Threading Alignment (mandatory)

- **Contracts produced (owned)**:
  - `ES-C03`: Safe default advertising — built-in backends MUST NOT advertise
    `agent_api.exec.external_sandbox.v1` by default; externally sandboxed hosts opt-in explicitly via
    backend config `allow_external_sandbox_exec` (canonical: `docs/specs/universal-agent-api/contract.md`).
    - Produced by: `S1` (Codex conformance) + `S2` (Claude Code conformance).
- **Contracts consumed**:
  - `ES-C01`: External sandbox execution policy extension key — the key id
    `agent_api.exec.external_sandbox.v1` and its “validated before spawn” semantics (owned by SEAM-1;
    canonical: `docs/specs/universal-agent-api/extensions-spec.md`).
    - Consumed by: `S1`/`S2` (advertising/allowlisting the key string only; validation/mapping is
      owned by SEAM-3/SEAM-4).
- **Dependency edges honored**:
  - `SEAM-1 blocks SEAM-2`: this plan assumes SEAM-1 has pinned the key semantics and R0/validation
    ordering before exposing the capability via built-in backends.
  - `SEAM-2 blocks SEAM-3`: `S1` ensures Codex mapping is only reachable behind explicit host opt-in.
  - `SEAM-2 blocks SEAM-4`: `S2` ensures Claude mapping is only reachable behind explicit host opt-in.
- **Parallelization notes**:
  - What can proceed now: `S1` and `S2` can be developed in parallel after SEAM-1 is stable (touch
    surfaces do not overlap).
  - What must wait: SEAM-3 depends on `S1`; SEAM-4 depends on `S2`; SEAM-5 depends on SEAM-3/4.

