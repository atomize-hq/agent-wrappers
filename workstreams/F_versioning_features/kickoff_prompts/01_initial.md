You are starting Workstream F (Versioning + Feature Detection), Task F1-design-capability-model.

Branch/worktree workflow (follow before coding, see workstreams/KICKOFF_GUIDE.md):
1) Checkout the workstream branch: `git checkout ws/F_versioning_features`.
2) In `workstreams/F_versioning_features/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/F_versioning_features/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/F1-design-capability-model`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-F1 task/F1-design-capability-model` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: design capability/version model and probing strategy (codex --version, features list, help parsing) with caching keyed by binary path.
Resources: workstreams/F_versioning_features/BRIEF.md, workstreams/F_versioning_features/tasks.json, existing code in crates/codex/src/lib.rs.
Deliverable: design note/doc comments committed to repo.

Completion steps (in this order, see workstreams/KICKOFF_GUIDE.md):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (run tests as needed).
2) Return to the workstream branch `ws/F_versioning_features`: `git checkout ws/F_versioning_features`.
3) Merge the task branch: `git merge --no-ff task/F1-design-capability-model`.
4) Remove the worktree: `git worktree remove ../wt-F1` (optional but recommended).
5) In `workstreams/F_versioning_features/tasks.json`, update this task status to "done" (or equivalent).
6) Update `workstreams/F_versioning_features/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/F_versioning_features/kickoff_prompts/<next>.md` (create the file) while on the workstream branch, following the guide.
