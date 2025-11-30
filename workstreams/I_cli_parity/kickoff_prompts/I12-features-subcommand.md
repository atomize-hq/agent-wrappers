You are starting Workstream I (CLI Parity), Task I12-features-subcommand.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/I_cli_parity`.
2) In `workstreams/I_cli_parity/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/I_cli_parity/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/I12-features-subcommand`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-I12 task/I12-features-subcommand` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Expose `codex features list` as a first-class wrapper (JSON/text), alongside existing capability probes; add docs/tests and note behavior.
Resources: workstreams/I_cli_parity/BRIEF.md, workstreams/I_cli_parity/tasks.json, crates/codex/src/lib.rs, README.md, CLI_MATRIX.md.
Grounding (from Codex CLI):
- `codex features list` prints a table (feature name, stage, enabled state). Stages: experimental/beta/stable/deprecated/removed.
- No subcommand-specific flags; global config/profile/overrides can change effective state. Exits zero on success.
Deliverable: Code/docs/tests adding the features subcommand wrapper; committed on the task branch.

Completion steps (in this order):
1) In the worktree on the task branch: finish code/docs/tests, `git status`, `git add ...`, `git commit -m "<msg>"` (run tests as needed).
2) Return to the workstream branch: `git checkout ws/I_cli_parity`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/I12-features-subcommand`.
4) Remove the worktree if you created one: `git worktree remove ../wt-I12` (optional but recommended).
5) Update `workstreams/I_cli_parity/tasks.json` to mark the task "done".
6) Update `workstreams/I_cli_parity/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/I_cli_parity/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
