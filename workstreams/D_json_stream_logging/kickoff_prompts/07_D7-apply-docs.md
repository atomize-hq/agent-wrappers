You are starting Workstream D (JSON Streaming + Logging), Task D7-apply-docs - Document apply/diff helpers.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/D_json_stream_logging`.
2) In `workstreams/D_json_stream_logging/tasks.json`, add/mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/D_json_stream_logging/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/D7-apply-docs`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-D7 task/D7-apply-docs` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Document the new apply/diff helper surface, including usage with streaming/log tee defaults and what artifacts are returned (stdout/stderr/exit). Add a short example/snippet if helpful.
Resources: workstreams/D_json_stream_logging/BRIEF.md, README.md, crates/codex/src/lib.rs, recent tests/examples.
Deliverable: Updated docs (and optional example) that show how to call apply/diff helpers and what to expect, noting RUST_LOG default behavior.

Completion steps (in this order):
1) In the worktree on the task branch: finish code/docs, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/D_json_stream_logging`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/D7-apply-docs`.
4) Remove the worktree if you created one: `git worktree remove ../wt-D7` (optional but recommended).
5) Update `workstreams/D_json_stream_logging/tasks.json` to mark the task "done".
6) Update `workstreams/D_json_stream_logging/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/D_json_stream_logging/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
