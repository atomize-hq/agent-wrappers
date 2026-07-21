---
name: orchestrate-workers
description: Lead a Codex-first documentation, implementation, and review workflow, with an Opus adversarial reviewer only as a parallel review lane.
---

# Orchestrate Workers

Use this skill when a lead Claude Code session needs delegated documentation or
implementation work. The lead session is the architect and integrator: it
defines the plan and direction before dispatch, assigns bounded ownership,
normalizes review evidence, and gives the final candidate a direct once-over.

## Roles and hard boundaries

| Role | Permitted work |
| --- | --- |
| Lead orchestrator | Repository investigation, plan and direction, task boundaries, integration, review adjudication, final source/diff/verification once-over. |
| `codex-profile-worker` | All delegated documentation, implementation, and Codex review work, under an explicit profile. |
| `opus-adversarial-reviewer` | Read-only adversarial review only, launched in parallel with a Codex review lane. Never documentation or implementation. |

Do not delegate architecture ownership to either worker. Do not launch the
Opus reviewer for an implementation or documentation packet, and do not launch
it as a standalone substitute for Codex review.

## Lead planning gate

Before every dispatch, inspect the repository and write a concise internal plan
that identifies:

1. objective, intended user-visible or contract outcome, and explicit non-goals;
2. base revision, affected interfaces, invariants, dependencies, and acceptance
   conditions;
3. a non-overlapping write-owner map; and
4. the review intent: the specific risks, contracts, and changed areas that a
   later review must examine.

If these are not known, investigate or ask the user; do not send an open-ended
worker request such as “finish the feature” or “review everything.”

## Required Codex task packet

Every Codex packet must contain all sections below. `Work type` is exactly one
of `documentation`, `implementation`, or `review`.

```md
# Task packet: <short name>

## Work type
documentation | implementation | review

## Objective
<one concrete, measurable outcome>

## Base revision
<commit SHA and expected starting state>

## Allowed write set
- `<path or glob>` (use `None` for review)

## Required interfaces and invariants
- <must remain true>

## Constraints and non-goals
- <scope, no-go areas, commit policy>

## Dependencies
- <completed prerequisite, or `None`>

## Verification commands
```sh
<exact commands>
```

## Required return evidence
- Changed-file list and diff summary
- Commands run with pass/fail output
- Unresolved risks or deviations
- Commit hash, only when requested
```

Also state `Codex profile: atomize_systems_azure` (or another explicit
profile). Invoke the launcher with `--sandbox workspace-write` for
documentation or implementation, and `--sandbox read-only` for review.

## Documentation and implementation loop

1. Dispatch documentation and implementation only to `codex-profile-worker`.
2. Parallelize only independent packets with distinct write sets and worktrees.
3. Inspect each returned status, diff, and verification result. Worker prose is
   evidence, not approval.
4. Integrate deliberately and verify in the canonical integration worktree.
5. Start review only against a known candidate revision and the lead's stated
   review intent.

## Required parallel review loop

For every candidate requiring review, dispatch these **at the same time**
against the same base/candidate revision, contract paths, changed-file set, and
review intent:

1. `codex-profile-worker` with `Work type: review`, `Allowed write set: None`,
   an explicit Codex profile, and `--sandbox read-only`.
2. `opus-adversarial-reviewer` with a read-only packet and the same evidence.

The Codex lane checks defects, edge cases, missing tests, and simpler safe
alternatives. The Opus lane tries to falsify the candidate through concrete
failure scenarios. Neither reviewer may redesign the work or claim final
approval.

## Lead review adjudication and final validation

When both review results return, the lead must inspect the candidate, relevant
contracts, and each cited location directly. For every finding, record one of:

- **accepted** — materially violates the objective, invariant, or review intent;
- **rejected** — unsupported, already addressed, out of scope, or beyond the
  stated intent; include a brief reason; or
- **deferred** — real but deliberately outside this packet; identify the
  follow-up boundary without silently widening the current work.

The lead must also verify that the combined findings are coherent: they address
the intended candidate and right contract areas, do not conflict with one
another, and do not overreach into a redesign or unrelated cleanup. Accepted
findings go to a new bounded `codex-profile-worker` remediation packet. Re-run
the parallel review loop when remediation changes material behavior or risks.

Only after the review result is clean may the lead make the final once-over:

1. inspect the final diff and changed files against the original objective and
   non-goals;
2. read the relevant source and contracts directly for integration coherence;
3. run the required canonical verification in the integration worktree; and
4. confirm no accepted review finding, scope leak, or unverified claim remains.

Do not declare completion based only on clean reviewer reports or passing
automation.
