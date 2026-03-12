# Contradictions Audit Report

## Meta
- Generated at: 2026-03-12T19:34:17Z
- Files audited: 49
- Scan used: no

## Summary
- Total issues: 1
- By severity: blocker=1, critical=0, major=0, minor=0
- High-confidence contradictions: 1

## Issue index
| ID | Severity | Confidence | Type | Subject | Files |
|---|---|---|---|---|---|
| CX-0001 | blocker | high | api | Codex fork transport for agent_api.exec.add_dirs.v1 | docs/specs/universal-agent-api/extensions-spec.md; docs/specs/codex-app-server-jsonrpc-contract.md |

## Issues

### CX-0001 — Codex fork transport for `agent_api.exec.add_dirs.v1`
- Severity: blocker
- Confidence: high
- Type: api
- Subject: Codex fork transport for agent_api.exec.add_dirs.v1
- Scope: environment=`all`, version=`v1`, feature_flag=`none`, timeline=`current`
- Statement A: `docs/specs/universal-agent-api/extensions-spec.md:245-247` — “Backends MUST honor the same effective add-dir set for new-session, resume, and forked runs...”
- Statement B: `docs/specs/codex-app-server-jsonrpc-contract.md:109-117` — `thread/fork` only pins `threadId`, `cwd`, `approvalPolicy`, `sandbox`, and `persistExtendedHistory`.
- Why this conflicts: the universal extension contract requires accepted add-dir requests to survive fork flows, but the canonical Codex fork transport has no field that can carry the add-dir set into `thread/fork` or `turn/start`. The current docs therefore describe a required behavior with no canonical transport that can realize it.
- What must be true:
  - The canon must define the exact Codex fork transport for accepted add-dir inputs, or explicitly pin a safe rejection path instead of implicit support.
  - The planning pack, ADR, and Codex backend contract must converge on the same fork behavior before implementation starts.
- Suggested evidence order:
  - codebase
  - tests
  - other-docs
  - decision
