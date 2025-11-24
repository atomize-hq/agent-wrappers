You are starting Workstream E (MCP + App Server), Task E10-app-runtime-api.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/E_mcp_app_server`.
2) In `workstreams/E_mcp_app_server/tasks.json`, add this task entry if missing and mark it as "doing" while on the workstream branch.
3) Log session start in `workstreams/E_mcp_app_server/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/E10-app-runtime-api`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-E10 task/E10-app-runtime-api` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Expose a lightweight API surface around the app-server helpers so callers can list available runtimes/metadata and prepare launch-ready stdio configs (Workstream A env prep intact) without mutating stored config or thread state.

Resources: workstreams/E_mcp_app_server/BRIEF.md, workstreams/E_mcp_app_server/tasks.json, BACKLOG.md, crates/codex/src/mcp.rs.

Deliverable: public app-server runtime helpers + tests that mirror the MCP runtime manager ergonomics (stdio prep, metadata listing) without altering persisted definitions.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/E_mcp_app_server`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/E10-app-runtime-api`.
4) Remove the worktree if you created one: `git worktree remove ../wt-E10` (optional but recommended).
5) Update `workstreams/E_mcp_app_server/tasks.json` to mark the task "done".
6) Update `workstreams/E_mcp_app_server/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/E_mcp_app_server/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
