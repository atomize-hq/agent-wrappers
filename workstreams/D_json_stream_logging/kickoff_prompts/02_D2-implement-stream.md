You are starting Workstream D (JSON Streaming + Logging), Task D2-implement-stream - Implement JSONL streaming and parsing.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/D_json_stream_logging`.
2) In `workstreams/D_json_stream_logging/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/D_json_stream_logging/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/D2-implement-stream`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-D2 task/D2-implement-stream` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Implement async streaming API that spawns `codex exec --json`, parses each line into typed events, flushes in real time, and surfaces idle timeout errors. Ensure ANSI is disabled when parsing. Include support for `--output-last-message` path handling and capture apply/diff stdout/stderr/exit.
Resources: crates/codex/src/lib.rs, workstreams/D_json_stream_logging/BRIEF.md.
Deliverable: Streaming API implementation plus JSON parsing hooked up to the new event types.

Completion steps (in this order):
1) In the worktree on the task branch: finish code, `git status`, `git add ...`, `git commit -m "<msg>"` (run `cargo test -p codex` as needed).
2) Return to the workstream branch: `git checkout ws/D_json_stream_logging`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/D2-implement-stream`.
4) Remove the worktree if you created one: `git worktree remove ../wt-D2` (optional but recommended).
5) Update `workstreams/D_json_stream_logging/tasks.json` to mark the task "done".
6) Update `workstreams/D_json_stream_logging/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/D_json_stream_logging/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
