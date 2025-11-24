You are starting Workstream E (MCP + App Server), Task E7-mcp-runtime-launchers.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/E_mcp_app_server`.
2) In `workstreams/E_mcp_app_server/tasks.json`, add this task entry if missing and mark it as "doing" while on the workstream branch.
3) Log session start in `workstreams/E_mcp_app_server/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/E7-mcp-runtime-launchers`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-E7 task/E7-mcp-runtime-launchers` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Hook resolved MCP runtime configs into launcher/connector helpers that can spin up stdio servers (using Workstream A env prep) or connect to streamable HTTP endpoints with headers/timeouts applied. Surface tool enable/disable hints alongside the runtime configs.
Resources: workstreams/E_mcp_app_server/BRIEF.md, workstreams/E_mcp_app_server/tasks.json, BACKLOG.md, crates/codex/src/mcp.rs.
Deliverable: runtime launcher/connector helpers plus tests covering env merges, timeouts, and tool hint propagation.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/E_mcp_app_server`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/E7-mcp-runtime-launchers`.
4) Remove the worktree if you created one: `git worktree remove ../wt-E7` (optional but recommended).
5) Update `workstreams/E_mcp_app_server/tasks.json` to mark the task "done".
6) Update `workstreams/E_mcp_app_server/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/E_mcp_app_server/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
