You are starting Workstream F (Versioning + Feature Detection), Task F13-tbd (fill in the actual task name before starting).

Before you proceed, add the new task to `workstreams/F_versioning_features/tasks.json` with ID `F13-...`, mark it "doing", and confirm dependencies/outputs. Update `BRIEF.md` with the agreed scope if it is not already captured.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/F_versioning_features`.
2) In `workstreams/F_versioning_features/tasks.json`, ensure the F13 task exists (add it if missing) and mark it as "doing" while on the workstream branch.
3) Log session start in `workstreams/F_versioning_features/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/F13-<name>`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-F13 task/F13-<name>` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Define the F13 scope with the PM/tech lead (e.g., post-release follow-ups, additional host guidance, or backlog cleanup), then execute it once captured in `tasks.json` and `BRIEF.md`.

Resources: workstreams/F_versioning_features/BRIEF.md, workstreams/F_versioning_features/tasks.json, crates/codex/src/lib.rs, crates/codex/README.md, crates/codex/EXAMPLES.md, prior kickoff prompts for workflow details.
Deliverable: Per `tasks.json` once the task is defined.

Completion steps (in this order):
1) In the worktree on the task branch: finish code/docs, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/F_versioning_features`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/F13-<name>`.
4) Remove the worktree if you created one: `git worktree remove ../wt-F13` (optional but recommended).
5) Update `workstreams/F_versioning_features/tasks.json` to mark the task "done".
6) Update `workstreams/F_versioning_features/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/F_versioning_features/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
