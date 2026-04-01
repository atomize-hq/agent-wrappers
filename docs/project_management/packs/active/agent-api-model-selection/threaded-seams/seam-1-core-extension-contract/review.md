---
seam_id: SEAM-1
review_phase: pre_exec
execution_horizon: active
basis_ref: seam.md#basis
---
# Review Bundle - SEAM-1 Core extension key contract

This artifact feeds `gates.pre_exec.review`.
`../../review_surfaces.md` is pack orientation only.

## Falsification questions

- Can downstream seams still cite a "pass" record that is only a provisional local `git HEAD` reference?
- Can canonical sources disagree (extensions spec vs registry entry vs run protocol) without opening a blocking remediation?
- Can ADR/pack restatements drift from the canonical specs in a way that would mislead implementation seams?

## R1 - Canonical authority flow

```mermaid
flowchart LR
  Spec["Canonical specs (docs/specs/**)"] --> ADR["ADR-0020 (rationale)"]
  Spec --> Pack["This pack (planning restatement)"]
  ADR --> Pack
  Pack --> Gate["THR-01 verification record (published reference)"]
  Gate --> Downstream["Downstream seams implement + advertise"]
```

## R2 - Evidence and gate record

```mermaid
flowchart TB
  Compare["Compare canonical sources + ADR + pack restatement"] --> Pass["pass: no unresolved canonical-doc delta"]
  Compare --> Fail["fail: canonical-doc delta opened"]
  Pass --> Ref["Publish commit/PR reference"]
  Ref --> Unblock["Unblock SEAM-2/3/4/5"]
```

## Likely mismatch hotspots

- registry entry drifts from owner spec (bucket or capability id)
- invalid-request template changes in one doc but not others
- run-protocol terminal Error event rule drifts from pack restatement

## Pre-exec findings

- Revalidation: canonical sources and ADR/pack restatements still align on v1 semantics (trim-before-validate, UTF-8
  byte bound `1..=128`, absence behavior, exact InvalidRequest template, and terminal error-event rule) as of
  2026-04-01 local working tree.
- Note: SEAM-1 verification record still cites a provisional local `git HEAD` reference; a published commit/PR
  reference is preferred for downstream citation, but canonical alignment is the approval criterion for this pack.

## Pre-exec gate disposition

- **Review gate**: passed
- **Contract gate**: passed (canonical alignment approved; commit/PR reference preferred but not required)
- **Revalidation gate**: passed
- **Opened remediations**: none

## Planned seam-exit gate focus

- **What must be true before downstream promotion is legal**: SEAM-1 verification record cites a commit/PR reference and any drift is resolved in canonical specs first.
- **Which outbound contracts/threads matter most**: `THR-01` and `C-01..C-04`.
- **Which review-surface deltas would force downstream revalidation**: any edit to the canonical spec sections governing v1 semantics.
