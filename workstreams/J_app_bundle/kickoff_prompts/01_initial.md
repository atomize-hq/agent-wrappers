You are starting Workstream J (Bundled Binary & Home Isolation), Task J1-bundled-binary-design.

Branch/worktree workflow (follow before coding, see workstreams/KICKOFF_GUIDE.md):
1) Checkout the workstream branch: `git checkout ws/J_app_bundle` (create it from main if missing).
2) In `workstreams/J_app_bundle/tasks.json`, mark this task as "doing" (edit JSON) while on the workstream branch.
3) Log session start in `workstreams/J_app_bundle/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/J1-bundled-binary-design`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-J1 task/J1-bundled-binary-design` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Design the opt-in app-bundled Codex binary flow that never relies on the user's global Codex install. Specify bundle layout + version pin/update expectations, how host apps should pick per-project CODEX_HOME roots and place auth files, and how the wrapper should expose a helper without changing current defaults.
Resources: workstreams/J_app_bundle/BRIEF.md, workstreams/J_app_bundle/tasks.json, crates/codex/src/lib.rs, README.md.
Deliverable: a design note/doc comments committed to the repo describing the bundling/home/auth plan and API surface.

Completion steps (in this order, see workstreams/KICKOFF_GUIDE.md):
1) In the worktree on the task branch: finish design, `git status`, `git add ...`, `git commit -m "<msg>"`.
2) Return to the workstream branch `ws/J_app_bundle`: `git checkout ws/J_app_bundle`.
3) Merge the task branch: `git merge --no-ff task/J1-bundled-binary-design`.
4) Remove the worktree: `git worktree remove ../wt-J1` (optional but recommended).
5) In `workstreams/J_app_bundle/tasks.json`, update this task status to "done".
6) Update `workstreams/J_app_bundle/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/J_app_bundle/kickoff_prompts/<next>.md` while on the workstream branch.
