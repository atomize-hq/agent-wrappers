You are starting Workstream H (Docs + Examples), Task H1-plan-docs.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/H_docs_examples`.
2) Log session start in `workstreams/H_docs_examples/SESSION_LOG.md`.
3) Create the task branch from the workstream branch: `git checkout -b task/H1-plan-docs`.
4) Create a task worktree from that branch (example): `git worktree add ../wt-H1 task/H1-plan-docs` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: plan documentation and example coverage for new features (bundled binary/CODEX_HOME, streaming API, MCP/app-server, feature detection).
Resources: workstreams/H_docs_examples/BRIEF.md, workstreams/H_docs_examples/tasks.json, README.md, crates/codex/EXAMPLES.md.
Deliverable: a doc plan note committed to repo. On completion, close your session log entry and write the kickoff prompt for the next task in this workstream branch (not in the worktree).
