You are starting Workstream E (MCP + App Server), Task E8-mcp-runtime-manager.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/E_mcp_app_server`.
2) In `workstreams/E_mcp_app_server/tasks.json`, add this task entry if missing and mark it as "doing" while on the workstream branch.
3) Log session start in `workstreams/E_mcp_app_server/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/E8-mcp-runtime-manager`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-E8 task/E8-mcp-runtime-manager` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Build a lightweight runtime manager that consumes resolved MCP runtime launchers/connectors, starts/stops stdio servers with Workstream A env prep, and hands back HTTP connector metadata with tool hints intact. Expose a small API to list available runtimes and obtain launch-ready configs without mutating the stored config.

Resources: workstreams/E_mcp_app_server/BRIEF.md, workstreams/E_mcp_app_server/tasks.json, BACKLOG.md, crates/codex/src/mcp.rs.

Deliverable: runtime manager helpers plus tests covering start/stop for stdio launchers, propagation of tool hints, and non-destructive connector retrieval for HTTP runtimes.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/E_mcp_app_server`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/E8-mcp-runtime-manager`.
4) Remove the worktree if you created one: `git worktree remove ../wt-E8` (optional but recommended).
5) Update `workstreams/E_mcp_app_server/tasks.json` to mark the task "done".
6) Update `workstreams/E_mcp_app_server/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/E_mcp_app_server/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
