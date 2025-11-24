You are starting Workstream A (Binary + Env Isolation), Task A6-release-polish - Polish and release the binary/env isolation work.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/A_binary_env`.
2) In `workstreams/A_binary_env/tasks.json`, add this task if needed and mark it as "doing" while on the workstream branch.
3) Log session start in `workstreams/A_binary_env/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/A6-release-polish`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-A6 task/A6-release-polish` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Finalize release polish for the binary/CODEX_HOME isolation work (docs, metadata, versioning) so the host apps can adopt the new helpers cleanly.
Resources: workstreams/A_binary_env/BRIEF.md, workstreams/A_binary_env/tasks.json, crates/codex/src/lib.rs, crates/codex/EXAMPLES.md, crates/codex/examples, crates/codex/tests.
Deliverable: Release-ready docs/metadata for binary/env isolation (e.g., README/notes or version bumps as needed) plus passing tests.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch `ws/A_binary_env`: `git checkout ws/A_binary_env`.
3) Merge the task branch: `git merge --no-ff task/A6-release-polish`.
4) Remove the worktree if you created one: `git worktree remove ../wt-A6` (optional but recommended).
5) Update `workstreams/A_binary_env/tasks.json` to mark this task as done.
6) Update `workstreams/A_binary_env/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/A_binary_env/kickoff_prompts/<next>.md` while on the workstream branch (follow this guide).
