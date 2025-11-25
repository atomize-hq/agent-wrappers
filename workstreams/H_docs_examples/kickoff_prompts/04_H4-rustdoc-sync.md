You are starting Workstream H (Docs + Examples), Task H4-rustdoc-sync.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/H_docs_examples`.
2) In `workstreams/H_docs_examples/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/H_docs_examples/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/H4-rustdoc-sync`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-H4 task/H4-rustdoc-sync` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: sync crate-level docs/rustdoc with the latest APIs and examples (binary/CODEX_HOME env prep, streaming event shapes, log tee + artifacts, MCP/app-server flows, capability detection hooks). Add short inline examples where helpful and link back to README/EXAMPLES.
Resources: workstreams/H_docs_examples/BRIEF.md, workstreams/H_docs_examples/tasks.json, README.md, crates/codex/EXAMPLES.md, crates/codex/src/lib.rs, crates/codex/examples/, workstreams D/E/F outputs for event/capability shapes.
Deliverable: refreshed rustdoc/module docs and any supporting notes/tests needed to keep docs accurate.

Completion steps (in this order):
1) In the worktree on the task branch: finish docs, `git status`, `git add ...`, `git commit -m "<msg>"` (tests/doc tests as needed).
2) Return to the workstream branch: `git checkout ws/H_docs_examples`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/H4-rustdoc-sync`.
4) Remove the worktree if you created one: `git worktree remove ../wt-H4` (optional but recommended).
5) Update `workstreams/H_docs_examples/tasks.json` to mark the task "done".
6) Update `workstreams/H_docs_examples/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/H_docs_examples/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
