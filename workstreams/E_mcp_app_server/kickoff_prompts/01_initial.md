You are starting Workstream E (MCP + App Server), Task E1-design-mcp-app.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/E_mcp_app_server`.
2) Log session start in `workstreams/E_mcp_app_server/SESSION_LOG.md`.
3) Create the task branch from the workstream branch: `git checkout -b task/E1-design-mcp-app`.
4) Create a task worktree from that branch (example): `git worktree add ../wt-E1 task/E1-design-mcp-app` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: design APIs/lifecycle for spawning codex mcp-server and app-server over stdio, covering codex/codex-reply and app thread/turn flows.
Resources: workstreams/E_mcp_app_server/BRIEF.md, workstreams/E_mcp_app_server/tasks.json, DeepWiki notes in BACKLOG.md.
Deliverable: design note/doc comments committed to repo; note reliance on Workstream A env-prep for spawning. On completion, close your session log entry and write the kickoff prompt for the next task in this workstream branch (not in the worktree).
