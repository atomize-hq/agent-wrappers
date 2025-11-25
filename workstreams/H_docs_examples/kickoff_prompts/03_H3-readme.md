You are starting Workstream H (Docs + Examples), Task H3-readme.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/H_docs_examples`.
2) In `workstreams/H_docs_examples/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/H_docs_examples/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/H3-readme`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-H3 task/H3-readme` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: update README and `crates/codex/EXAMPLES.md` to cover bundled binary/CODEX_HOME setup, streaming API usage, MCP/app-server usage, and version/feature detection hooks. Link to the new examples and call out safety defaults.
Resources: workstreams/H_docs_examples/BRIEF.md, workstreams/H_docs_examples/tasks.json, workstreams/H_docs_examples/H1-plan-docs.md, crates/codex/EXAMPLES.md, README.md, crates/codex/examples/.
Deliverable: refreshed README + EXAMPLES guide documenting the new APIs and examples.

Completion steps (in this order):
1) In the worktree on the task branch: finish docs, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/H_docs_examples`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/H3-readme`.
4) Remove the worktree if you created one: `git worktree remove ../wt-H3` (optional but recommended).
5) Update `workstreams/H_docs_examples/tasks.json` to mark the task "done".
6) Update `workstreams/H_docs_examples/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/H_docs_examples/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
