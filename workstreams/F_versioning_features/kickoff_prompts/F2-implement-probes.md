You are starting Workstream F (Versioning + Feature Detection), Task F2-implement-probes.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/F_versioning_features`.
2) In `workstreams/F_versioning_features/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/F_versioning_features/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/F2-implement-probes`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-F2 task/F2-implement-probes` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Implement the version and feature probes with caching per binary path; parse `codex --version`, prefer `codex features list --json` or text fallback, and parse `--help` as a last resort; reuse the env isolation helpers from Workstream A.
Resources: workstreams/F_versioning_features/BRIEF.md, crates/codex/src/lib.rs.
Deliverable: Probe implementation plus tests (see tasks.json), updating the capability model and cache logic.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/F_versioning_features`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/F2-implement-probes`.
4) Remove the worktree if you created one: `git worktree remove ../wt-F2` (optional but recommended).
5) Update `workstreams/F_versioning_features/tasks.json` to mark the task "done".
6) Update `workstreams/F_versioning_features/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/F_versioning_features/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
