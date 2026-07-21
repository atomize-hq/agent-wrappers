---
name: cross-review
description: Run Codex and Opus adversarial read-only review lanes in parallel, then require lead-session scope-aware adjudication and final validation.
---

# Cross Review

Use this skill only after the lead has a known candidate revision and an
explicit review intent. It is the review phase of `$orchestrate-workers`, not
a replacement for planning or implementation.

## Prepare the shared review packet

The lead first supplies both lanes the same:

- base and candidate revisions;
- objective, acceptance criteria, and explicit non-goals;
- contract or requirement paths;
- changed-file set and exact review areas;
- allowed read-only verification commands; and
- review intent: the risks and behavior the review is meant to validate.

Reviewers must assess the supplied candidate, not current uncommitted work or
an imagined broader project.

## Dispatch the two lanes concurrently

1. Dispatch `codex-profile-worker` with `Work type: review`, an explicit Codex
   profile, `Allowed write set: None`, and `--sandbox read-only`. Require it to
   preserve Codex's final response separately from observed repository evidence.
2. At the same time, dispatch `opus-adversarial-reviewer` with the identical
   candidate evidence and a read-only packet.

The Codex reviewer looks for concrete defects, edge cases, missing tests, and
safe simplifications. The Opus reviewer actively tries to falsify the candidate
through requirement violations and realistic failure scenarios. Neither lane
may write, redesign, broaden scope, or self-approve the candidate.

## Lead adjudication

The lead reads the source, diff, and cited contracts directly before deciding
any finding. Normalize each as accepted, rejected, or deferred, with rationale.
Reject findings that are unsupported, unrelated to the stated review intent,
or that widen the packet into redesign or unrelated cleanup. Preserve genuine
out-of-scope issues as explicit follow-ups rather than silently absorbing them.

Send accepted findings only to a new, bounded Codex remediation packet. If the
remediation changes material behavior or the reviewed risk surface, re-run both
review lanes concurrently.

## Clean exit

After both lanes are clean and all accepted findings are resolved, the lead must
perform a final once-over of the source, final diff, scope boundaries, and
canonical verification output. Completion requires that direct validation—not
just reviewer agreement or test success—confirms the candidate meets its intent.
