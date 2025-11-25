You are starting Workstream F (Versioning + Feature Detection), Task F11-capability-ttl-helper.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/F_versioning_features`.
2) In `workstreams/F_versioning_features/tasks.json`, ensure this task exists (add it if missing) and mark it as "doing" (edit the JSON) while on the workstream branch.
3) Log session start in `workstreams/F_versioning_features/SESSION_LOG.md`.
4) Create the task branch from the workstream branch: `git checkout -b task/F11-capability-ttl-helper`.
5) Create a task worktree from that branch (example): `git worktree add ../wt-F11 task/F11-capability-ttl-helper` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: Add a helper that enforces a TTL/backoff on capability snapshots using `CodexCapabilities.collected_at`, especially for environments where fingerprints are missing/unreliable (FUSE/overlays). Surface a simple API hosts can call to decide when to force `CapabilityCachePolicy::Refresh`/`Bypass`, with docs and tests covering TTL expiry, hot-swap reuse, and metadata-missing paths.

Resources: workstreams/F_versioning_features/BRIEF.md, workstreams/F_versioning_features/tasks.json, crates/codex/src/lib.rs.
Deliverable: Per `tasks.json` once the task is defined (code/tests/docs as appropriate).

Completion steps (in this order):
1) In the worktree on the task branch: finish code/docs, `git status`, `git add ...`, `git commit -m "<msg>"` (tests as needed).
2) Return to the workstream branch: `git checkout ws/F_versioning_features`.
3) Merge the task branch into the workstream branch: `git merge --no-ff task/F11-capability-ttl-helper`.
4) Remove the worktree if you created one: `git worktree remove ../wt-F11` (optional but recommended).
5) Update `workstreams/F_versioning_features/tasks.json` to mark the task "done".
6) Update `workstreams/F_versioning_features/SESSION_LOG.md` with end time/outcome.
7) Write the kickoff prompt for the next task in `workstreams/F_versioning_features/kickoff_prompts/<next>.md` (follow this guide); do this while on the workstream branch.
