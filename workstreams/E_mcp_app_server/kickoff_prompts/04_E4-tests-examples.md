You are starting Workstream E (MCP + App Server), Task E4-tests-examples.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/E_mcp_app_server`.
2) In `workstreams/E_mcp_app_server/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/E_mcp_app_server/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/E4-tests-examples`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-E4 task/E4-tests-examples` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: add integration-style tests and examples for MCP/app-server flows (codex/codex-reply, thread/start/resume, turn/start/interrupt) with cancellation and notification coverage.
Resources: workstreams/E_mcp_app_server/BRIEF.md, workstreams/E_mcp_app_server/tasks.json, crates/codex/src/mcp.rs, existing fake stdio helpers.
Deliverable: tests/examples demonstrating MCP/app-server usage with task_complete/item events and cancellation handling; code committed to repo.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/E_mcp_app_server`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/E4-tests-examples`.
4) Remove the worktree if you created one: `git worktree remove ../wt-E4` (optional but recommended).
5) Update `workstreams/E_mcp_app_server/tasks.json` to mark the task "done".
6) Update `workstreams/E_mcp_app_server/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/E_mcp_app_server/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
