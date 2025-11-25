You are starting Workstream A (Binary + Env Isolation), Task A7-release-publish - Ship the binary/env isolation release.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/A_binary_env`.
2) In `workstreams/A_binary_env/tasks.json`, add this task if needed and mark it as "doing" while on the workstream branch.
3) Log session start in `workstreams/A_binary_env/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/A7-release-publish`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-A7 task/A7-release-publish` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Tag and publish the codex crate with binary/CODEX_HOME isolation (v0.2.x), including release notes/changelog entries and any publishing metadata needed for host apps.
Resources: workstreams/A_binary_env/BRIEF.md, workstreams/A_binary_env/tasks.json, workstreams/A_binary_env/SESSION_LOG.md, crates/codex/README.md, crates/codex/src/lib.rs, Cargo.toml, Cargo.lock.
Deliverable: Release-ready artifacts (tag/changelog/readme validation) plus passing `cargo test -p codex`; publish or document how to publish if credentials are unavailable.

Completion steps (in this order):
1) In the worktree on the task branch: finish code/docs, `git status`, `git add ...`, `git commit -m "<msg>"` (run `cargo test -p codex`).
2) Return to the workstream branch: `git checkout ws/A_binary_env`.
3) Merge the task branch: `git merge --no-ff task/A7-release-publish`.
4) Remove the worktree if you created one: `git worktree remove ../wt-A7` (optional but recommended).
5) Update `workstreams/A_binary_env/tasks.json` to mark this task as done.
6) Update `workstreams/A_binary_env/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/A_binary_env/kickoff_prompts/<next>.md` while on the workstream branch (follow this guide).
