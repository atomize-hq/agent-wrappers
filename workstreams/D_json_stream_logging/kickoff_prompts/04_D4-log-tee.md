You are starting Workstream D (JSON Streaming + Logging), Task D4-log-tee - File logging for streamed events.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/D_json_stream_logging`.
2) In `workstreams/D_json_stream_logging/tasks.json`, add/mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/D_json_stream_logging/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/D4-log-tee`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-D4 task/D4-log-tee` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Add opt-in log teeing for JSON streaming so clients can persist raw event lines without breaking stdout/stderr controls, and ensure `RUST_LOG` handling stays correct. Include tests that validate tee ordering/flush behavior and log contents for apply/diff/tool traffic.
Resources: crates/codex/src/lib.rs, workstreams/D_json_stream_logging/BRIEF.md, existing streaming tests/examples.
Deliverable: Logging surface (builder/request options) with passing tests showing file teeing works alongside console mirroring, plus brief docs/example updates.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/D_json_stream_logging`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/D4-log-tee`.
4) Remove the worktree if you created one: `git worktree remove ../wt-D4` (optional but recommended).
5) Update `workstreams/D_json_stream_logging/tasks.json` to mark the task "done".
6) Update `workstreams/D_json_stream_logging/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/D_json_stream_logging/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
