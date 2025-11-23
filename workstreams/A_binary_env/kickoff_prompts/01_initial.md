You are starting Workstream A (Binary + Env Isolation), Task A1-design-env-api.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/A_binary_env`.
2) Log session start in `workstreams/A_binary_env/SESSION_LOG.md`.
3) Create the task branch from the workstream branch: `git checkout -b task/A1-design-env-api`.
4) Create a task worktree from that branch (example): `git worktree add ../wt-A1 task/A1-design-env-api` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: design how the crate accepts a pinned Codex binary path and an app-scoped CODEX_HOME per invocation, and how all Command spawns use a shared env-prep helper.
Resources: workstreams/A_binary_env/BRIEF.md, workstreams/A_binary_env/tasks.json, crates/codex/src/lib.rs.
Deliverable: an API proposal (doc comments or short design note in-repo) that keeps backward compatibility and outlines directory expectations under CODEX_HOME. On completion, close your session log entry and write the kickoff prompt for the next task in this workstream branch (not in the worktree).
