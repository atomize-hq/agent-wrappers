# Handoff: ADR-0013 backend harness (agent_api) + open-set agent kind unblocker

## Session Metadata
- Created: 2026-02-22 17:39:12
- Project: /Users/spensermcconnell/__Active_Code/codex-wrapper
- Branch: staging
- Session duration: ~1h

### Recent Commits (for context)
  - 0e5ec17 Document backend harness spec
  - ef28be6 Document tools facet schema
  - e5765ab Fix: commit blocked queue updates
  - 3b8be3b Merge branch 'main' into staging
  - f4a3423 CI: generate PR bodies as scratch files (#66)

## Handoff Chain

- **Continues from**: None (fresh start)
- **Supersedes**: None

> This is the first handoff for this task.

## Current State Summary

Validated that the Universal Agent API “baseline norm” work is in place (Codex + Claude advertise
`agent_api.events.live`, `agent_api.exec.non_interactive`, `agent_api.tools.{results,structured}.v1`,
and `agent_api.artifacts.final_text.v1`). Landed/confirmed the unblocker for onboarding future
agents without semver churn in `wrapper_events` by making `WrapperAgentKind` an open set via
`Other(String)` and updating the ingestion contract wording. Drafted ADR-0013 for an internal
`agent_api` “backend harness” module (implementation-only refactor) intended to shrink per-backend
adapter files and make future CLI onboarding mostly “spawn + parse + map”. Next work is to
implement the harness and migrate `agent_api` backends onto it with no behavior changes.

## Codebase Understanding

### Architecture Overview

Key layering to preserve:
- Per-agent wrapper crates (`crates/codex`, `crates/claude_code`, future `crates/<agent>`): spawn +
  parse streaming output into typed events + completion handle.
- Universal facade (`crates/agent_api`): capability/extension gating, request validation, env merge,
  timeouts, bounds enforcement, DR-0012 completion gating, and mapping typed events into the stable
  `AgentWrapperEvent` envelope.

The proposed backend harness lives in `crates/agent_api` and centralizes the repeated glue
concerns currently duplicated across `crates/agent_api/src/backends/*.rs`. It is not a new spec and
should not change the public API; it should just enforce existing normative rules from
`docs/specs/universal-agent-api/*` by construction.

### Critical Files

| File | Purpose | Relevance |
|------|---------|-----------|
| /Users/spensermcconnell/__Active_Code/codex-wrapper/docs/specs/universal-agent-api/contract.md | Core universal contract (env precedence, extension gating) | Harness must enforce these invariants |
| /Users/spensermcconnell/__Active_Code/codex-wrapper/docs/specs/universal-agent-api/run-protocol-spec.md | DR-0012 completion vs stream finality rules | Harness must preserve gating semantics |
| /Users/spensermcconnell/__Active_Code/codex-wrapper/docs/specs/universal-agent-api/event-envelope-schema-spec.md | Bounds + “no raw backend lines” + tools facet schema | Harness must enforce bounds centrally |
| /Users/spensermcconnell/__Active_Code/codex-wrapper/docs/specs/universal-agent-api/capability-matrix.md | Current parity snapshot | Confirms baseline parity at `agent_api.*` |
| /Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/backends/codex.rs | Codex universal backend adapter | Candidate for harness adoption/refactor |
| /Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/backends/claude_code.rs | Claude universal backend adapter | Candidate for harness adoption/refactor |
| /Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/run_handle_gate.rs | DR-0012 gating implementation | Harness should use existing gating helper |
| /Users/spensermcconnell/__Active_Code/codex-wrapper/crates/agent_api/src/bounds.rs | Event/completion bounds enforcement | Harness should call these APIs |
| /Users/spensermcconnell/__Active_Code/codex-wrapper/docs/adr/0013-agent-api-backend-harness.md | ADR describing harness approach | Source-of-truth for this initiative |
| /Users/spensermcconnell/__Active_Code/codex-wrapper/crates/wrapper_events/src/normalized.rs | Minimal normalized envelope for ingestion crate | Unblocks future agents via `Other(String)` |

### Key Patterns Discovered

- ADRs have an `ADR_BODY_SHA256` drift guard; after edits run `make adr-fix ADR=...` and verify
  with `make adr-check ADR=...`.
- Specs under `docs/specs/**` are normative; ADRs are rationale/supporting context.
- Capability ids are an open-set of strings; bucketing is a naming convention defined in
  `docs/specs/universal-agent-api/capabilities-schema-spec.md`.

## Work Completed

### Tasks Finished

- [x] Confirmed baseline parity at the `agent_api.*` layer via capability matrix + tests.
- [x] Implemented/confirmed open-set agent kind in `wrapper_events` via `WrapperAgentKind::Other(String)`.
- [x] Updated wrapper-events ingestion contract wording to match the open-set agent kind requirement.
- [x] Added ADR-0013 describing the internal `agent_api` backend harness module and fixed ADR hash.
- [x] Ran `cargo test -p wrapper_events --all-features`.

### Files Modified

| File | Changes | Rationale |
|------|---------|-----------|
| /Users/spensermcconnell/__Active_Code/codex-wrapper/crates/wrapper_events/src/normalized.rs | Added `WrapperAgentKind::Other(String)` and removed `Copy` | Avoid semver churn when onboarding new agent kinds |
| /Users/spensermcconnell/__Active_Code/codex-wrapper/docs/specs/wrapper-events-ingestion-contract.md | Clarified `WrapperAgentKind` is an open set | Keep spec aligned with implementation |
| /Users/spensermcconnell/__Active_Code/codex-wrapper/docs/adr/0013-agent-api-backend-harness.md | New ADR describing harness approach | Record decision/rationale + validation plan |

### Decisions Made

| Decision | Options Considered | Rationale |
|----------|-------------------|-----------|
| Backend harness is internal-only (no new core spec) | Add new capability ids/spec vs internal refactor | Keep universal contract stable; harness enforces existing normative rules |
| Keep one crate per agent CLI | Centralize into `agent_api` | Preserves clear responsibility: wrapper crates parse/spawn; universal maps/enforces invariants |

## Pending Work

### Immediate Next Steps

1. Implement the new internal `agent_api` backend harness module (new file under
   `crates/agent_api/src/`; name TBD) to centralize: extension allowlist validation-before-spawn,
   env merge precedence, timeout wrapping, drain-on-drop forwarding pattern, and bounds enforcement
   hooks.
2. Migrate `crates/agent_api/src/backends/codex.rs` onto the harness with no behavior changes; run
   `make test` (or at least `cargo test -p agent_api --all-features`).
3. Migrate `crates/agent_api/src/backends/claude_code.rs` onto the harness; ensure DR-0012 gating
   semantics remain identical and tooling facet/final_text behavior stays stable.

### Blockers/Open Questions

- [ ] Where should the harness live and how opinionated should it be (single generic “run loop” vs
      a few helper functions)? Keep it audit-friendly (avoid macros).
- [ ] There is an untracked planning pack directory:
      `/Users/spensermcconnell/__Active_Code/codex-wrapper/docs/project_management/packs/active/agent-api-backend-harness/`
      Decide whether it should be committed, moved, or deleted.

### Deferred Items

- Expanding new bucket capabilities (`agent_api.control.*`, `agent_api.obs.*`, etc.) was deferred
  until after the harness to reduce duplicated glue across backends.

## Context for Resuming Agent

### Important Context

- The harness is NOT a new spec or capability id. The current core specs already define the rules
  the harness should enforce (extensions fail-closed + validate-before-spawn; env precedence;
  bounds; DR-0012 completion gating). Do not add new `agent_api.*` ids just to justify the harness.
- Baseline parity at `agent_api.*` is already reflected in
  `/Users/spensermcconnell/__Active_Code/codex-wrapper/docs/specs/universal-agent-api/capability-matrix.md`.
  Backend-specific capabilities still differ (expected).
- ADR hash guard: any ADR edits require `make adr-fix ADR=...`.

### Assumptions Made

- Harness adoption should be “no behavior change”; existing backend tests define expected behavior.
- Per-backend adapters remain (thin) because spawn + mapping are inherently backend-specific.

### Potential Gotchas

- DR-0012 completion gating is subtle: `completion` must not resolve until stream finality is
  observed (or stream dropped). Preserve the existing `run_handle_gate` usage.
- Redaction/bounds: do not accidentally reintroduce raw backend line content into `Error.message`
  while centralizing parsing/forwarding.
- Be careful not to expand accepted `extensions` keys; validation must remain strict and occur
  before spawning processes.

## Environment State

### Tools/Services Used

- `cargo test -p wrapper_events --all-features`
- `make adr-fix ADR=docs/adr/0013-agent-api-backend-harness.md`
- `make adr-check ADR=docs/adr/0013-agent-api-backend-harness.md`
- `rg`, `sed`, `git`

### Active Processes

- None

### Environment Variables

- None set/required for this work (tests use standard cargo env).

## Related Resources

- ADR: `/Users/spensermcconnell/__Active_Code/codex-wrapper/docs/adr/0013-agent-api-backend-harness.md`
- Seam artifacts (currently untracked): `/Users/spensermcconnell/__Active_Code/codex-wrapper/docs/project_management/packs/active/agent-api-backend-harness/README.md`
- Baseline normative writeup: `/Users/spensermcconnell/__Active_Code/codex-wrapper/docs/project_management/baseline_norm.md`

---

**Security Reminder**: Before finalizing, run `validate_handoff.py` to check for accidental secret exposure.
