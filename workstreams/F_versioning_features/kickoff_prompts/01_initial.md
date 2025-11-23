You are starting Workstream F (Versioning + Feature Detection), Task F1-design-capability-model.

Branch/worktree workflow (follow before coding):
1) Checkout the workstream branch: `git checkout ws/F_versioning_features`.
2) Log session start in `workstreams/F_versioning_features/SESSION_LOG.md`.
3) Create the task branch from the workstream branch: `git checkout -b task/F1-design-capability-model`.
4) Create a task worktree from that branch (example): `git worktree add ../wt-F1 task/F1-design-capability-model` and do all code in the worktree. Do **not** edit docs/logs inside the worktree.

Task goal: design capability/version model and probing strategy (codex --version, features list, help parsing) with caching keyed by binary path.
Resources: workstreams/F_versioning_features/BRIEF.md, workstreams/F_versioning_features/tasks.json, existing code in crates/codex/src/lib.rs.
Deliverable: design note/doc comments committed to repo. On completion, close your session log entry and write the kickoff prompt for the next task in this workstream branch (not in the worktree).
