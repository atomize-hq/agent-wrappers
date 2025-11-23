You are starting Workstream E (MCP + App Server), Task E1-design-mcp-app.

Branch/worktree workflow (follow before coding, see workstreams/KICKOFF_GUIDE.md):
1) Checkout the workstream branch: `git checkout ws/E_mcp_app_server`.
2) In `workstreams/E_mcp_app_server/tasks.json`, mark this task as \"doing\" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/E_mcp_app_server/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/E1-design-mcp-app`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-E1 task/E1-design-mcp-app` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: design APIs/lifecycle for spawning codex mcp-server and app-server over stdio, covering codex/codex-reply and app thread/turn flows.
Resources: workstreams/E_mcp_app_server/BRIEF.md, workstreams/E_mcp_app_server/tasks.json, DeepWiki notes in BACKLOG.md.
Deliverable: design note/doc comments committed to repo; note reliance on Workstream A env-prep for spawning.

Completion steps (in this order, see workstreams/KICKOFF_GUIDE.md):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m \"<msg>\"` (run tests as needed).
2) Return to the workstream branch `ws/E_mcp_app_server`: `git checkout ws/E_mcp_app_server`.
3) Merge the task branch: `git merge --no-ff task/E1-design-mcp-app`.
4) Remove the worktree: `git worktree remove ../wt-E1` (optional but recommended).
5) In `workstreams/E_mcp_app_server/tasks.json`, update this task status to \"done\" (or equivalent).
6) Update `workstreams/E_mcp_app_server/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/E_mcp_app_server/kickoff_prompts/<next>.md` (create the file) while on the workstream branch, following the guide.
