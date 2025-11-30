You are starting Workstream I (CLI Parity), Task I12-cli-parity-followups.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/I_cli_parity`.
2) In `workstreams/I_cli_parity/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/I_cli_parity/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/I12-cli-parity-followups`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-I12 task/I12-cli-parity-followups` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Triage and close the remaining CLI parity gaps called out in I11: decide whether to surface CLI `--oss` and top-level `--enable/--disable` feature toggles (beyond sandbox) in the Rust wrapper, and whether to wrap or explicitly document `codex cloud exec` and the shell-completion helper. Implement the needed API/doc/test changes or record explicit deferrals.
Resources: workstreams/I_cli_parity/BRIEF.md, workstreams/I_cli_parity/tasks.json, CLI_MATRIX.md, README.md, crates/codex/EXAMPLES.md, crates/codex/src/lib.rs.
Deliverable: Code/doc/test updates (or a clear follow-up note) that resolve the outstanding gaps; include test results in the summary.

Completion steps (in this order):
1) In the worktree on the task branch: finish code/docs/tests, `git status`, `git add ...`, `git commit -m "<msg>"`.
2) Return to the workstream branch: `git checkout ws/I_cli_parity`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/I12-cli-parity-followups`.
4) Remove the worktree if you created one: `git worktree remove ../wt-I12` (optional but recommended).
5) Update `workstreams/I_cli_parity/tasks.json` to mark the task "done" or note follow-ups.
6) Update `workstreams/I_cli_parity/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/I_cli_parity/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
