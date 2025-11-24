You are starting Workstream D (JSON Streaming + Logging), Task D5-docs-release - Document streaming logging surface.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/D_json_stream_logging`.
2) In `workstreams/D_json_stream_logging/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/D_json_stream_logging/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/D5-docs-release`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-D5 task/D5-docs-release` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Polish the JSON streaming/log tee documentation: update crate-level docs/README to describe the API, log tee options, and RUST_LOG defaults, and capture any short release notes for the new surface.
Resources: README.md, workstreams/D_json_stream_logging/BRIEF.md, crates/codex/src/lib.rs, existing examples/tests.
Deliverable: Clear docs showing how to consume streams and enable log teeing (with defaults/limits called out), plus release note/update as needed.

Completion steps (in this order):
1) In the worktree on the task branch: finish docs/code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/D_json_stream_logging`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/D5-docs-release`.
4) Remove the worktree if you created one: `git worktree remove ../wt-D5` (optional but recommended).
5) Update `workstreams/D_json_stream_logging/tasks.json` to mark the task "done".
6) Update `workstreams/D_json_stream_logging/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/D_json_stream_logging/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
