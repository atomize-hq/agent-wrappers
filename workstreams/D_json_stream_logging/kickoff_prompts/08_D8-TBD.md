You are starting Workstream D (JSON Streaming + Logging), Task D8-TBD - **fill in the concrete task name/goal before starting**.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/D_json_stream_logging`.
2) In `workstreams/D_json_stream_logging/tasks.json`, add/mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/D_json_stream_logging/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/D8-TBD`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-D8 task/D8-TBD` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: TODO — replace with the specific D8 objective and any constraints once defined.
Resources: TODO — list the files/notes that should be consulted.
Deliverable: TODO — describe what must be produced for the task to be considered complete.

Completion steps (in this order):
1) In the worktree on the task branch: finish code/docs, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/D_json_stream_logging`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/D8-TBD`.
4) Remove the worktree if you created one: `git worktree remove ../wt-D8` (optional but recommended).
5) Update `workstreams/D_json_stream_logging/tasks.json` to mark the task "done".
6) Update `workstreams/D_json_stream_logging/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/D_json_stream_logging/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
