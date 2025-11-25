You are starting Workstream A (Binary + Env Isolation), Task A2-implement-env-prep.

Branch/worktree workflow (follow before coding, see workstreams/KICKOFF_GUIDE.md):
1) Checkout the workstream branch: `git checkout ws/A_binary_env`.
2) In `workstreams/A_binary_env/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/A_binary_env/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/A2-implement-env-prep`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-A2 task/A2-implement-env-prep` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: implement builder fields and the shared env-prep helper so CODEX_HOME/binary overrides apply per Command spawn without mutating the parent environment.
Resources: workstreams/A_binary_env/BRIEF.md, workstreams/A_binary_env/tasks.json, crates/codex/src/lib.rs.
Deliverable: code changes compiled (tests as needed).

Completion steps (in this order, see workstreams/KICKOFF_GUIDE.md):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (run tests as needed, e.g., `cargo test -p codex`).
2) Return to the workstream branch `ws/A_binary_env`: `git checkout ws/A_binary_env`.
3) Merge the task branch: `git merge --no-ff task/A2-implement-env-prep`.
4) Remove the worktree: `git worktree remove ../wt-A2` (optional but recommended).
5) In `workstreams/A_binary_env/tasks.json`, update this task status to "done" (or equivalent).
6) Update `workstreams/A_binary_env/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/A_binary_env/kickoff_prompts/<next>.md` (create the file) while on the workstream branch, following the guide.
