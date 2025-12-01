You are starting Workstream J (Bundled Binary & Home Isolation), Task J3-bundled-binary-docs-examples.

Branch/worktree workflow (follow before coding, see workstreams/KICKOFF_GUIDE.md):
1) Checkout the workstream branch: `git checkout ws/J_app_bundle` (create it from main if missing).
2) In `workstreams/J_app_bundle/tasks.json`, mark this task as "doing" (edit JSON) while on the workstream branch.
3) Log session start in `workstreams/J_app_bundle/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/J3-bundled-binary-docs-examples`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-J3 task/J3-bundled-binary-docs-examples` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Document the bundled binary + isolated `CODEX_HOME` flow with examples. Show how to resolve a pinned bundled binary via the new helper, pick per-project homes, and place/copy auth files safely. Keep current defaults intact; make the bundled pattern clearly additive/recommended.
Resources: workstreams/J_app_bundle/BRIEF.md, workstreams/J_app_bundle/tasks.json, README.md, crates/codex/EXAMPLES.md, crates/codex/examples/.
Tests: cargo test -p codex (if time permits).

Completion steps (in this order, see workstreams/KICKOFF_GUIDE.md):
1) In the worktree on the task branch: add docs/examples, `git status`, `git add ...`, `git commit -m "<msg>"` (update or add tests/examples as needed).
2) Return to the workstream branch `ws/J_app_bundle`: `git checkout ws/J_app_bundle`.
3) Merge the task branch: `git merge --no-ff task/J3-bundled-binary-docs-examples`.
4) Remove the worktree: `git worktree remove ../wt-J3` (optional but recommended).
5) In `workstreams/J_app_bundle/tasks.json`, update this task status to "done".
6) Update `workstreams/J_app_bundle/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/J_app_bundle/kickoff_prompts/<next>.md` while on the workstream branch.
