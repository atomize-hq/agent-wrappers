You are starting Workstream I (CLI Parity), Task I1-design-parity-apis.

Branch/worktree workflow (follow before coding, see workstreams/KICKOFF_GUIDE.md):
1) Checkout the workstream branch: `git checkout ws/I_cli_parity` (or create it from main if missing).
2) In `workstreams/I_cli_parity/tasks.json`, mark this task as "doing" (edit JSON) while on the workstream branch.
3) Log session start in `workstreams/I_cli_parity/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/I1-design-parity-apis`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-I1 task/I1-design-parity-apis` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: design builder/request APIs to cover all missing CLI flags and config overrides (config key/value, approval/sandbox, full-auto/yolo, cd, local-provider, search, resume last/all, reasoning/verbosity setters). Decide on defaults and compatibility.
Resources: workstreams/I_cli_parity/BRIEF.md, workstreams/I_cli_parity/tasks.json, crates/codex/src/lib.rs, CLI_MATRIX.md.
Deliverable: a design note/doc comments committed to the repo outlining the planned API and wiring.

Completion steps (in this order, see workstreams/KICKOFF_GUIDE.md):
1) In the worktree on the task branch: finish design, `git status`, `git add ...`, `git commit -m "<msg>"`.
2) Return to the workstream branch `ws/I_cli_parity`: `git checkout ws/I_cli_parity`.
3) Merge the task branch: `git merge --no-ff task/I1-design-parity-apis`.
4) Remove the worktree: `git worktree remove ../wt-I1` (optional but recommended).
5) In `workstreams/I_cli_parity/tasks.json`, update this task status to "done".
6) Update `workstreams/I_cli_parity/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/I_cli_parity/kickoff_prompts/<next>.md` while on the workstream branch.
