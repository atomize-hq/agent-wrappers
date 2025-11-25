You are starting Workstream I (CLI Parity), Task I3-tests-docs-examples.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/I_cli_parity`.
2) In `workstreams/I_cli_parity/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/I_cli_parity/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/I3-tests-docs-examples`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-I3 task/I3-tests-docs-examples` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Add unit tests for the new CLI parity flags/config overrides and update docs/examples for the new APIs. Verify backward compatibility and showcase usage.
Resources: workstreams/I_cli_parity/BRIEF.md, workstreams/I_cli_parity/design_notes/I1-design-parity-apis.md, CLI_MATRIX.md, crates/codex/src/lib.rs, README.md, crates/codex/EXAMPLES.md, crates/codex/examples/.
Deliverable: Tests and docs/examples updated to cover the new builder/request overrides; compatibility verified.

Completion steps (in this order):
1) In the worktree on the task branch: finish code/docs/tests, `git status`, `git add ...`, `git commit -m "<msg>"` (run tests as needed).
2) Return to the workstream branch: `git checkout ws/I_cli_parity`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/I3-tests-docs-examples`.
4) Remove the worktree if you created one: `git worktree remove ../wt-I3` (optional but recommended).
5) Update `workstreams/I_cli_parity/tasks.json` to mark the task "done".
6) Update `workstreams/I_cli_parity/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/I_cli_parity/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
