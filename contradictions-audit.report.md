# Contradictions Audit Report

## Meta
- Generated at: 2026-03-03T02:22:28Z
- Files audited: 43
- Scan used: yes (`/tmp/contradictions-audit.scan.json`)

## Summary
- Total issues: 1
- By severity: blocker=0, critical=0, major=1, minor=0
- High-confidence contradictions: 1

## Issue index
| ID | Severity | Confidence | Type | Subject | Files |
|---|---|---|---|---|---|
| CX-0001 | major | high | terminology | `agent_api.exec.external_sandbox.v1` intended host isolation boundary (internal vs external sandboxing) | docs/adr/0016-universal-agent-api-bounded-backend-config-pass-through.md; docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/scope_brief.md |

## Issues

### CX-0001 — Internal vs external sandboxed hosts
- Severity: major
- Confidence: high
- Type: terminology
- Subject: `agent_api.exec.external_sandbox.v1` intended host isolation boundary (internal vs external sandboxing)
- Scope: environment=all, version=v1, feature_flag=unknown, timeline=planned
- Statement A: `docs/adr/0016-universal-agent-api-bounded-backend-config-pass-through.md:100-102` — “…allowing internally sandboxed hosts (e.g. Substrate) to opt into explicitly dangerous exec policy via `agent_api.exec.external_sandbox.v1`.”
- Statement B: `docs/project_management/packs/active/agent-api-external-sandbox-exec-policy/scope_brief.md:9-10` — “…to let externally sandboxed hosts (e.g., Substrate) explicitly request that a built-in backend relax internal approvals/sandbox/permissions guardrails…”
- Why this conflicts: The docs describe the same dangerous key as being intended for both “internally sandboxed” and “externally sandboxed” hosts. These phrases imply opposite trust boundaries (where the isolation lives), which affects when it is safe to relax internal guardrails. Without a single consistent term/definition, implementers and integrators can infer incompatible safety assumptions.
- What must be true:
  - Define the single intended trust boundary for `agent_api.exec.external_sandbox.v1`: the sandbox/isolation is provided by the host environment external to the backend process (or explicitly state a different boundary and align naming/wording across docs).
- Suggested evidence order:
  - codebase
  - tests
  - runtime-config
  - git-history
  - other-docs
  - external
  - decision
