You are starting Workstream D (JSON Streaming + Logging), Task D6-apply-artifacts - Capture apply/diff outputs.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/D_json_stream_logging`.
2) In `workstreams/D_json_stream_logging/tasks.json`, add/mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/D_json_stream_logging/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/D6-apply-artifacts`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-D6 task/D6-apply-artifacts` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Capture apply/diff artifacts from Codex so callers receive stdout/stderr/exit status when applying changes. Ensure this integrates cleanly with existing streaming/log tee behavior and honors `RUST_LOG` defaults.
Resources: crates/codex/src/lib.rs, workstreams/D_json_stream_logging/BRIEF.md, existing streaming/logging tests/examples.
Deliverable: Apply/diff helper surface with tests covering success/failure exit codes, stdout/stderr capture, and any interactions with JSON streaming/log teeing.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/D_json_stream_logging`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/D6-apply-artifacts`.
4) Remove the worktree if you created one: `git worktree remove ../wt-D6` (optional but recommended).
5) Update `workstreams/D_json_stream_logging/tasks.json` to mark the task "done".
6) Update `workstreams/D_json_stream_logging/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/D_json_stream_logging/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
