You are starting Workstream H (Docs + Examples), Task H1-plan-docs.

Branch/worktree workflow (follow before coding, see workstreams/KICKOFF_GUIDE.md):
1) Checkout the workstream branch: `git checkout ws/H_docs_examples`.
2) In `workstreams/H_docs_examples/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/H_docs_examples/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/H1-plan-docs`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-H1 task/H1-plan-docs` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: plan documentation and example coverage for new features (bundled binary/CODEX_HOME, streaming API, MCP/app-server, feature detection).
Resources: workstreams/H_docs_examples/BRIEF.md, workstreams/H_docs_examples/tasks.json, README.md, crates/codex/EXAMPLES.md.
Deliverable: a doc plan note committed to repo.

Completion steps (in this order, see workstreams/KICKOFF_GUIDE.md):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (run tests as needed).
2) Return to the workstream branch `ws/H_docs_examples`: `git checkout ws/H_docs_examples`.
3) Merge the task branch: `git merge --no-ff task/H1-plan-docs`.
4) Remove the worktree: `git worktree remove ../wt-H1` (optional but recommended).
5) In `workstreams/H_docs_examples/tasks.json`, update this task status to "done" (or equivalent).
6) Update `workstreams/H_docs_examples/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/H_docs_examples/kickoff_prompts/<next>.md` (create the file) while on the workstream branch, following the guide.
