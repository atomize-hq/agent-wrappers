You are starting Workstream I (CLI Parity), Task I13-proxy-uds-wrappers.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/I_cli_parity`.
2) In `workstreams/I_cli_parity/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/I_cli_parity/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/I13-proxy-uds-wrappers`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-I13 task/I13-proxy-uds-wrappers` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Wrap `codex responses-api-proxy` (API key–injecting proxy to /v1/responses) and `codex stdio-to-uds <socket_path>` (stdio relay to Unix domain sockets) with minimal APIs, docs, and smoke examples.
Resources: workstreams/I_cli_parity/BRIEF.md, workstreams/I_cli_parity/tasks.json, CLI_MATRIX.md, crates/codex/src/lib.rs, README.md, Codex CLI docs.
Grounding (from Codex CLI):
- `codex responses-api-proxy` reads the API key from stdin. Flags: `--port <PORT>` (default ephemeral on 127.0.0.1), `--server-info <FILE>` (write JSON `{port,pid}`), `--http-shutdown` (enable GET /shutdown → exit 0), `--upstream-url <URL>` (default https://api.openai.com/v1/responses). Only POST /v1/responses is proxied; others 403. Runs until shutdown; non-zero on startup/runtime errors.
- `codex stdio-to-uds <SOCKET_PATH>` relays stdin→UDS and UDS→stdout; exits when the connection closes or on error (non-zero).
Deliverable: Code/docs/tests adding these utility wrappers; committed on the task branch.

Completion steps (in this order):
1) In the worktree on the task branch: finish code/docs/tests, `git status`, `git add ...`, `git commit -m "<msg>"` (run tests as needed).
2) Return to the workstream branch: `git checkout ws/I_cli_parity`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/I13-proxy-uds-wrappers`.
4) Remove the worktree if you created one: `git worktree remove ../wt-I13` (optional but recommended).
5) Update `workstreams/I_cli_parity/tasks.json` to mark the task "done".
6) Update `workstreams/I_cli_parity/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/I_cli_parity/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
