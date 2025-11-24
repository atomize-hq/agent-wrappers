You are starting Workstream D (JSON Streaming + Logging), Task D3-tests-example - Tests and example for streaming.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/D_json_stream_logging`.
2) In `workstreams/D_json_stream_logging/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/D_json_stream_logging/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/D3-tests-example`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-D3 task/D3-tests-example` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Add tests covering JSONL streaming (event ordering, tool call parsing, idle timeout/error propagation) and create an example that streams events in real time while handling `--output-last-message` output. Update README or docs as needed.
Resources: crates/codex/src/lib.rs, crates/codex/examples, workstreams/D_json_stream_logging/BRIEF.md.
Deliverable: Passing tests for streaming behavior plus an example demonstrating consumption and last-message file handling.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (run `cargo test -p codex` as needed).
2) Return to the workstream branch: `git checkout ws/D_json_stream_logging`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/D3-tests-example`.
4) Remove the worktree if you created one: `git worktree remove ../wt-D3` (optional but recommended).
5) Update `workstreams/D_json_stream_logging/tasks.json` to mark the task "done".
6) Update `workstreams/D_json_stream_logging/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/D_json_stream_logging/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
