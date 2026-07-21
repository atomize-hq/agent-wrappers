---
name: cross-review
description: Run independent native-Claude and profile-pinned Codex read-only review lanes, then normalize findings in the lead session.
---

# Cross Review

Use this skill after a bounded candidate change exists. Both review lanes are
read-only and examine the same base/candidate diff. The lead session decides
whether a finding is valid and whether remediation is warranted.

## Inputs required from the lead

- Candidate base and head revisions.
- Requirement or contract paths.
- Exact files or diff under review.
- Verification commands reviewers may run.
- Explicit Codex review profile (for example, `workflows`).

## Lane A: native Claude reviewer

Dispatch the `opus-worker` with an allowed write set of `None` and an explicit
instruction not to modify files. Ask it to review requirements, architecture,
semantic correctness, integration risks, and missing tests. Its packet must
require each finding to include severity, file/line, rationale, and a concrete
failure mode.

## Lane B: Codex reviewer

Dispatch `codex-profile-worker` with the same candidate and a packet that
requires `--sandbox read-only`. Ask it to focus on defects, missing tests, edge
cases, regressions, and simpler alternatives. The launcher invocation must
include the designated profile and preserve Codex's final response with
`--output-last-message`.

## Normalize in the lead session

1. Deduplicate findings without treating agreement as proof.
2. Inspect the cited source and contract directly.
3. Record each finding as accepted, rejected (with rationale), or deferred.
4. For accepted findings, send a new bounded write packet to one owner.
5. Re-run review after remediation when risk or requirements warrant it.

Passing automated checks and reviewer self-reports are evidence, not a
substitute for source inspection by the lead.
