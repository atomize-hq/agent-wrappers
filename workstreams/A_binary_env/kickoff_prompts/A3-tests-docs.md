You are starting Workstream A (Binary + Env Isolation), Task A3-tests-docs - Tests and doc updates for env/binary overrides.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/A_binary_env`.
2) In `workstreams/A_binary_env/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/A_binary_env/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/A3-tests-docs`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-A3 task/A3-tests-docs` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Add unit tests for default binary + CODEX_BINARY override and CODEX_HOME override applied per command; add an example or README snippet showing bundled binary with an app-scoped CODEX_HOME.
Resources: workstreams/A_binary_env/BRIEF.md, workstreams/A_binary_env/tasks.json, crates/codex/src/lib.rs, crates/codex/examples.
Deliverable: New tests and docs/examples updates (run `cargo test -p codex` as needed).

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch `ws/A_binary_env`: `git checkout ws/A_binary_env`.
3) Merge the task branch: `git merge --no-ff task/A3-tests-docs`.
4) Remove the worktree if you created one: `git worktree remove ../wt-A3` (optional but recommended).
5) Update `workstreams/A_binary_env/tasks.json` to mark this task as done.
6) Update `workstreams/A_binary_env/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/A_binary_env/kickoff_prompts/<next>.md` while on the workstream branch (follow this guide).
