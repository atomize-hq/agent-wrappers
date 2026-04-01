# Remediation Log - Universal model selection (`agent_api.config.model.v1`)

## Open remediations

None.

## Resolved remediations

### REM-001 - SEAM-1 verification record is publishable (resolved)

```yaml
remediation_id: REM-001
origin_phase: pre_exec
source_gate: contract
related_seam: SEAM-1
related_slice: S2
related_thread: THR-01
related_contract: null
related_artifact: docs/project_management/packs/active/agent-api-model-selection/seam-1-core-extension-contract.md
severity: blocking
status: resolved
owner_seam: SEAM-1
blocked_targets:
  - seam: SEAM-1
    field: status
    value: exec-ready
summary: Previously, SEAM-1 verification record cited an unpublished local reference rather than a published commit/PR.
required_fix: Replace the unpublished local verification record reference with a published commit hash or PR URL once available.
resolution_evidence:
  - "2026-04-01: Resolved by commit fb2c17d, which replaces provisional refs with stable commit citations (verification record cites 4255d85 and 34b0ee9)."
  - "2026-04-01: Pack owner approved proceeding as long as canonical specs remain the normative source of truth and the pack/ADR align; commit/PR reference is preferred but not required for exec-ready."
```
