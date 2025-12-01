You are starting Workstream J (Bundled Binary & Home Isolation), Task J2-bundled-binary-helper.

Branch/worktree workflow (follow before coding, see workstreams/KICKOFF_GUIDE.md):
1) Checkout the workstream branch: `git checkout ws/J_app_bundle` (create it from main if missing).
2) In `workstreams/J_app_bundle/tasks.json`, mark this task as "doing" (edit JSON) while on the workstream branch.
3) Log session start in `workstreams/J_app_bundle/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/J2-bundled-binary-helper`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-J2 task/J2-bundled-binary-helper` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Implement the opt-in helper to resolve a pinned app-scoped Codex binary (bundle root + version), fail fast when missing, and keep behavior isolated from the user's global install. Keep default behavior unchanged unless the helper is used.
Resources: workstreams/J_app_bundle/BRIEF.md, workstreams/J_app_bundle/tasks.json, crates/codex/src/lib.rs, workstreams/J_app_bundle/design_notes/J1-bundled-binary-design.md.
Tests: cargo test -p codex (if time permits).

Completion steps (in this order, see workstreams/KICKOFF_GUIDE.md):
1) In the worktree on the task branch: implement the helper, `git status`, `git add ...`, `git commit -m "<msg>"` (add tests if applicable).
2) Return to the workstream branch `ws/J_app_bundle`: `git checkout ws/J_app_bundle`.
3) Merge the task branch: `git merge --no-ff task/J2-bundled-binary-helper`.
4) Remove the worktree: `git worktree remove ../wt-J2` (optional but recommended).
5) In `workstreams/J_app_bundle/tasks.json`, update this task status to "done".
6) Update `workstreams/J_app_bundle/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/J_app_bundle/kickoff_prompts/<next>.md` while on the workstream branch.
