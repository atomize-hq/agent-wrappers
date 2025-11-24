You are starting Workstream A (Binary + Env Isolation), Task A5-home-path-helpers - Expose CODEX_HOME layout helpers.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/A_binary_env`.
2) In `workstreams/A_binary_env/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/A_binary_env/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/A5-home-path-helpers`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-A5 task/A5-home-path-helpers` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Expose helpers that describe the CODEX_HOME layout (config/auth/.credentials/history/conversations/log paths) so host apps can discover files when using an app-scoped home. Respect create_home_dirs when materializing directories and avoid mutating the parent env.
Resources: workstreams/A_binary_env/BRIEF.md, workstreams/A_binary_env/tasks.json, crates/codex/src/lib.rs, crates/codex/EXAMPLES.md.
Deliverable: Public helper(s) and docs/examples showing CODEX_HOME layout discovery, plus tests covering path derivation and optional directory creation.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch `ws/A_binary_env`: `git checkout ws/A_binary_env`.
3) Merge the task branch: `git merge --no-ff task/A5-home-path-helpers`.
4) Remove the worktree if you created one: `git worktree remove ../wt-A5` (optional but recommended).
5) Update `workstreams/A_binary_env/tasks.json` to mark this task as done.
6) Update `workstreams/A_binary_env/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/A_binary_env/kickoff_prompts/<next>.md` while on the workstream branch (follow this guide).
