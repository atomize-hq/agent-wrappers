You are starting Workstream E (MCP + App Server), Task E3-implement-app-server.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/E_mcp_app_server`.
2) In `workstreams/E_mcp_app_server/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/E_mcp_app_server/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/E3-implement-app-server`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-E3 task/E3-implement-app-server` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: implement the app-server launcher and thread/turn helpers over stdio JSON-RPC, reusing the transport from E2.
Resources: workstreams/E_mcp_app_server/BRIEF.md, workstreams/E_mcp_app_server/tasks.json, BACKLOG.md notes, crates/codex/src/mcp.rs (transport from E2).
Deliverable: working app-server launcher with initialize + thread/start/resume and turn/start/interrupt calls, streaming task_complete/item notifications, cancellation support; code committed to repo.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/E_mcp_app_server`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/E3-implement-app-server`.
4) Remove the worktree if you created one: `git worktree remove ../wt-E3` (optional but recommended).
5) Update `workstreams/E_mcp_app_server/tasks.json` to mark the task "done".
6) Update `workstreams/E_mcp_app_server/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/E_mcp_app_server/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
