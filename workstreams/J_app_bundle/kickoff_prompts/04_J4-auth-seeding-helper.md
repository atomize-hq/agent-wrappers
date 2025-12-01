You are starting Workstream J (Bundled Binary & Home Isolation), Task J4-auth-seeding-helper.

Branch/worktree workflow (follow before coding, see workstreams/KICKOFF_GUIDE.md):
1) Checkout the workstream branch: `git checkout ws/J_app_bundle` (create it from main if missing).
2) In `workstreams/J_app_bundle/tasks.json`, mark this task as "doing" (edit JSON) while on the workstream branch.
3) Log session start in `workstreams/J_app_bundle/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/J4-auth-seeding-helper`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-J4 task/J4-auth-seeding-helper` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Add an opt-in helper to seed Codex auth into an app-scoped `CODEX_HOME` for bundled/isolated flows. Support copying `auth.json` and `.credentials.json` from a trusted seed home into a target home (no history/logs), surface clear errors when sources are missing or unsafe, and keep existing defaults untouched. Update docs/examples to show the helper alongside `resolve_bundled_binary` and per-project homes.
Resources: workstreams/J_app_bundle/BRIEF.md, workstreams/J_app_bundle/tasks.json, README.md, crates/codex/src/lib.rs, crates/codex/EXAMPLES.md, crates/codex/examples/.
Tests: cargo test -p codex (and add targeted tests around the new helper).

Completion steps (in this order, see workstreams/KICKOFF_GUIDE.md):
1) In the worktree on the task branch: finish code/docs/examples, `git status`, `git add ...`, `git commit -m "<msg>"` (update/add tests as needed).
2) Return to the workstream branch `ws/J_app_bundle`: `git checkout ws/J_app_bundle`.
3) Merge the task branch: `git merge --no-ff task/J4-auth-seeding-helper`.
4) Remove the worktree: `git worktree remove ../wt-J4` (optional but recommended).
5) In `workstreams/J_app_bundle/tasks.json`, update this task status to "done".
6) Update `workstreams/J_app_bundle/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/J_app_bundle/kickoff_prompts/<next>.md` while on the workstream branch.
