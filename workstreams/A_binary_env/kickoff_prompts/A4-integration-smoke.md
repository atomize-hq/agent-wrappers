You are starting Workstream A (Binary + Env Isolation), Task A4-integration-smoke - Integration smoke tests for env/binary overrides.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/A_binary_env`.
2) In `workstreams/A_binary_env/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/A_binary_env/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/A4-integration-smoke`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-A4 task/A4-integration-smoke` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Add end-to-end/smoke coverage proving CODEX_BINARY + CODEX_HOME are applied for every spawn site (exec, login/status/logout, MCP/app-server helpers). Use a fake codex binary to assert received env/args without hitting the real service.
Resources: workstreams/A_binary_env/BRIEF.md, workstreams/A_binary_env/tasks.json, crates/codex/src/lib.rs, existing tests/examples under crates/codex.
Deliverable: Integration-style tests (or harness) that exercise all spawn paths with overridden binary + CODEX_HOME, plus any supporting test fixtures. Run `cargo test -p codex` as needed.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch `ws/A_binary_env`: `git checkout ws/A_binary_env`.
3) Merge the task branch: `git merge --no-ff task/A4-integration-smoke`.
4) Remove the worktree if you created one: `git worktree remove ../wt-A4` (optional but recommended).
5) Update `workstreams/A_binary_env/tasks.json` to mark this task as done.
6) Update `workstreams/A_binary_env/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/A_binary_env/kickoff_prompts/<next>.md` while on the workstream branch (follow this guide).
