You are starting Workstream I (CLI Parity), Task I7-sandbox-command-design.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/I_cli_parity`.
2) In `workstreams/I_cli_parity/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/I_cli_parity/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/I7-sandbox-command-design`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-I7 task/I7-sandbox-command-design` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Design wrapper support for `codex sandbox` (aliases/subcommands, platform variants, post-run script behavior). Define API surface, mappings for positional/flags, and testing/doc approach. Capture limits or open questions.
Resources: workstreams/I_cli_parity/BRIEF.md, workstreams/I_cli_parity/tasks.json, CLI_MATRIX.md, crates/codex/src/lib.rs, README.md, Codex CLI help.
Deliverable: Design notes/plan committed (doc/kickoff or code comments) describing how to wrap `codex sandbox` and any follow-on tasks.

Completion steps (in this order):
1) In the worktree on the task branch: finish design note/updates, `git status`, `git add ...`, `git commit -m "<msg>"`.
2) Return to the workstream branch: `git checkout ws/I_cli_parity`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/I7-sandbox-command-design`.
4) Remove the worktree if you created one: `git worktree remove ../wt-I7` (optional but recommended).
5) Update `workstreams/I_cli_parity/tasks.json` to reflect the task status or add follow-on tasks.
6) Update `workstreams/I_cli_parity/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/I_cli_parity/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
