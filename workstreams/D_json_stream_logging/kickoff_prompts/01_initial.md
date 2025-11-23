You are starting Workstream D (JSON Streaming + Logging), Task D1-design-stream-types.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/D_json_stream_logging`.
2) Log session start in `workstreams/D_json_stream_logging/SESSION_LOG.md`.
3) Create the task branch from the workstream branch: `git checkout -b task/D1-design-stream-types`.
4) Create a task worktree from that branch (example): `git worktree add ../wt-D1 task/D1-design-stream-types` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: design typed JSONL event API and streaming surface for `codex exec --json`, covering thread/turn/item lifecycle and item variants.
Resources: workstreams/D_json_stream_logging/BRIEF.md, workstreams/D_json_stream_logging/tasks.json, crates/codex/src/lib.rs, DeepWiki notes in BACKLOG.md.
Deliverable: event type definitions and API sketch (doc comments or design note) committed to the repo. On completion, close your session log entry and write the kickoff prompt for the next task in this workstream branch (not in the worktree).
