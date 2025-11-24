You are starting Workstream F (Versioning + Feature Detection), Task F9-post-workstream-review.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/F_versioning_features`.
2) In `workstreams/F_versioning_features/tasks.json`, ensure this task exists (add it if missing) and mark it as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/F_versioning_features/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/F9-post-workstream-review`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-F9 task/F9-post-workstream-review` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Audit Workstream F's capability/versioning deliverables for gaps (docs/tests/cache guidance) and capture any remaining follow-ups (e.g., release notes, host integration notes, or backlog items) so the stream can be cleanly handed off.

Resources: workstreams/F_versioning_features/BRIEF.md, workstreams/F_versioning_features/tasks.json, crates/codex/src/lib.rs.
Deliverable: Per `tasks.json` once the task is defined (code/tests/docs/backlog updates as appropriate).

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/F_versioning_features`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/F9-post-workstream-review`.
4) Remove the worktree if you created one: `git worktree remove ../wt-F9` (optional but recommended).
5) Update `workstreams/F_versioning_features/tasks.json` to mark the task "done".
6) Update `workstreams/F_versioning_features/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/F_versioning_features/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
