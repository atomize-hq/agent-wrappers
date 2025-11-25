You are starting Workstream E (MCP + App Server), Task E5-mcp-management.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/E_mcp_app_server`.
2) In `workstreams/E_mcp_app_server/tasks.json`, add this task entry if missing and mark it as "doing" while on the workstream branch.
3) Log session start in `workstreams/E_mcp_app_server/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/E5-mcp-management`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-E5 task/E5-mcp-management` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: implement MCP config management commands (list/get/add/remove/login/logout) with JSON definitions and env injection on add. Support `[mcp_servers]` entries for stdio and `streamable_http` transports (headers, bearer env vars, timeouts, tool enable/disable), aligning with backlog item 1.
Resources: workstreams/E_mcp_app_server/BRIEF.md, workstreams/E_mcp_app_server/tasks.json, BACKLOG.md (Workstream E), crates/codex/src/mcp.rs, existing config/env helpers from Workstream A.
Deliverable: API/helpers covering MCP server config CRUD and auth flows with JSON-serializable inputs/outputs; code committed to repo.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/E_mcp_app_server`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/E5-mcp-management`.
4) Remove the worktree if you created one: `git worktree remove ../wt-E5` (optional but recommended).
5) Update `workstreams/E_mcp_app_server/tasks.json` to mark the task "done".
6) Update `workstreams/E_mcp_app_server/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/E_mcp_app_server/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
