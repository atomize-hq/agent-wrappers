You are starting Workstream I (CLI Parity), Task I11-execpolicy-wrapper.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/I_cli_parity`.
2) In `workstreams/I_cli_parity/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/I_cli_parity/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/I11-execpolicy-wrapper`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-I11 task/I11-execpolicy-wrapper` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Add a wrapper for `codex execpolicy check` (policy paths, pretty output, command argv) so hosts can validate shell commands against Starlark policies.
Resources: workstreams/I_cli_parity/BRIEF.md, workstreams/I_cli_parity/tasks.json, crates/codex/src/lib.rs, README.md, CLI_MATRIX.md, Codex execpolicy docs.
Grounding (from Codex CLI):
- `codex execpolicy check --policy <PATH>... [--pretty] -- <COMMAND...>` evaluates Starlark `.codexpolicy` files (repeatable `--policy` merges rules).
- Output is always JSON: match includes `decision` (allow/prompt/forbidden) and matched rules; no match → `{"noMatch":{}}`. Severity ordering: forbidden > prompt > allow; rule `decision` defaults to allow.
- Exits zero on successful evaluation; non-zero on errors.
Deliverable: Code/docs/tests adding execpolicy support; committed on the task branch.

Completion steps (in this order):
1) In the worktree on the task branch: finish code/docs/tests, `git status`, `git add ...`, `git commit -m "<msg>"` (run tests as needed).
2) Return to the workstream branch: `git checkout ws/I_cli_parity`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/I11-execpolicy-wrapper`.
4) Remove the worktree if you created one: `git worktree remove ../wt-I11` (optional but recommended).
5) Update `workstreams/I_cli_parity/tasks.json` to mark the task "done".
6) Update `workstreams/I_cli_parity/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/I_cli_parity/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
