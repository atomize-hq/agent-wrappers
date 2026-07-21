---
name: orchestrate-workers
description: Delegate bounded, isolated implementation tasks to native Claude Code or profile-pinned Codex workers while retaining lead-session integration authority.
---

# Orchestrate Workers

Use this skill when a lead Claude Code session needs parallel or delegated work.
The lead retains architectural decisions, integration, and final verification.

## Before dispatch

1. Inspect the repository and identify the base revision, acceptance criteria,
   dependencies, and a non-overlapping write-owner map.
2. Divide work only where tasks are independent. Do not assign two write tasks
   the same file, worktree, or generated artifact.
3. Send a complete packet. Never ask a worker to “finish the feature”,
   “investigate and fix everything”, or infer acceptance criteria.

Use `opus-worker` for bounded implementation that benefits from strong
repository judgment. Use `codex-profile-worker` for work that must run through
Codex. The latter always requires a named Codex profile such as `atomize_systems_azure`.
Native Claude worker isolation is supplied by its `isolation: worktree` agent
configuration; do not bypass it by directing multiple workers into the lead
worktree.

## Required task packet

```md
# Task packet: <short name>

## Objective
<one concrete, measurable outcome>

## Base revision
<commit SHA and expected starting state>

## Allowed write set
- `<path or glob>`

## Required interfaces and invariants
- <must remain true>

## Constraints
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

For a Codex packet, also state `Codex profile: atomize_systems_azure` (or another
explicit profile). The launcher command must be equivalent to:

```sh
scripts/run-codex-worker.sh \
  --profile atomize_systems_azure \
  --sandbox workspace-write \
  --worktree "$PWD" \
  --packet /path/outside/the-worktree/packet.md \
  --output-last-message /path/outside/the-worktree/codex-final.md
```

## Dispatch and integration

1. Start independent tasks concurrently only after checking ownership.
2. Treat worker output as evidence to inspect, not as approval.
3. Inspect every worker's `git status` and diff. Integrate deliberately;
   cherry-pick only a reviewed commit or manually apply selected changes.
4. Run the full required verification in the canonical integration worktree.
5. If evidence conflicts, inspect source and requirements directly; the lead
   resolves the conflict.

## Non-negotiable rules

- Do not let workers change the integration worktree concurrently.
- Do not widen a packet after dispatch; issue a replacement packet instead.
- Do not claim completion until lead-side verification passes.
- Use read-only packets for design races and reviews unless the lead explicitly
  selects one proposal for implementation.
