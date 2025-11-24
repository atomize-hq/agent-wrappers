You are starting Workstream H (Docs + Examples), Task H10-docs-live-fixture.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/H_docs_examples`.
2) In `workstreams/H_docs_examples/tasks.json`, mark this task as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/H_docs_examples/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/H10-docs-live-fixture`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-H10 task/H10-docs-live-fixture` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Use the new integration fixture with a real Codex binary (or CI-friendly runner) to refresh streaming/resume/apply sample payloads. Update docs/examples if drift is detected and record any remaining gaps for capability caching or typed streaming APIs.
Resources: workstreams/H_docs_examples/BRIEF.md, workstreams/H_docs_examples/tasks.json, BACKLOG.md, HANDOFF.md, README.md, crates/codex/EXAMPLES.md, crates/codex/src/lib.rs, crates/codex/examples/, `crates/codex/examples/fixtures/*`, and any CLI event specs.
Deliverable: Updated fixtures/docs/examples that match live CLI output plus a note on residual drift or caching risks.

Completion steps (in this order):
1) In the worktree on the task branch: finish code/doc/fixture updates, `git status`, `git add ...`, `git commit -m "<msg>"` (include tests/doc tests as needed).
2) Return to the workstream branch: `git checkout ws/H_docs_examples`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/H10-docs-live-fixture`.
4) Remove the worktree if you created one: `git worktree remove ../wt-H10` (optional but recommended).
5) Update `workstreams/H_docs_examples/tasks.json` to mark the task "done".
6) Update `workstreams/H_docs_examples/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/H_docs_examples/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
