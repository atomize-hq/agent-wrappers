# Kickoff Prompt Guide

Purpose: Every task agent writes or updates a kickoff prompt for the next task. Use this format and closing steps to avoid mistakes.

Kickoff prompt structure (example):
```
You are starting Workstream <X>, Task <ID> - <Title>.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/<workstream>`.
2) In `workstreams/<workstream>/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/<workstream>/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/<task-id>`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-<short> task/<task-id>` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: <what to achieve>.
Resources: <files/briefs>.
Deliverable: <what to produce>.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/<workstream>`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/<task-id>`.
4) Remove the worktree if you created one: `git worktree remove ../wt-<short>` (optional but recommended).
5) Update `workstreams/<workstream>/tasks.json` to mark the task "done".
6) Update `workstreams/<workstream>/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/<workstream>/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
```
