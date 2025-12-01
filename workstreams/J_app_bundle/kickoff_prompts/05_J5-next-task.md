You are starting Workstream J (Bundled Binary & Home Isolation), Task J5-<next-task>.

Branch/worktree workflow (follow before coding, see workstreams/KICKOFF_GUIDE.md):
1) Checkout the workstream branch: `git checkout ws/J_app_bundle` (create it from main if missing).
2) If the task is not yet in `workstreams/J_app_bundle/tasks.json`, add it with status "todo" and details from the backlog/BRIEF, then mark it "doing" while on the workstream branch.
3) Log session start in `workstreams/J_app_bundle/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/J5-<next-task>`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-J5 task/J5-<next-task>` and do all code in the worktree. Do **not** edit workstream logs/docs inside the worktree.

Task goal: Fill in from `tasks.json`/BRIEF for J5 (e.g., next hardening or follow-up for bundled binary/home isolation). Keep defaults untouched unless the task specifies otherwise.
Resources: workstreams/J_app_bundle/BRIEF.md, workstreams/J_app_bundle/tasks.json (after adding J5), README.md, crates/codex/EXAMPLES.md, crates/codex/examples/, crates/codex/src/lib.rs.
Tests: cargo test -p codex (plus task-specific tests).

Completion steps (in this order, see workstreams/KICKOFF_GUIDE.md):
1) In the worktree on the task branch: finish code/docs/examples, `git status`, `git add ...`, `git commit -m "<msg>"` (update/add tests as needed).
2) Return to the workstream branch `ws/J_app_bundle`: `git checkout ws/J_app_bundle`.
3) Merge the task branch: `git merge --no-ff task/J5-<next-task>`.
4) Remove the worktree: `git worktree remove ../wt-J5` (optional but recommended).
5) In `workstreams/J_app_bundle/tasks.json`, update this task status to "done".
6) Update `workstreams/J_app_bundle/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/J_app_bundle/kickoff_prompts/<next>.md` while on the workstream branch.
