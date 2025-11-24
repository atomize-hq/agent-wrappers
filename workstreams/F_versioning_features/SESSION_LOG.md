# Session Log — Workstream F (Versioning + Feature Detection)

Append entries: `[START ...] [END ...] Agent: <name> | Task(s): <IDs> | Branch: <branch> | Notes`. At task completion, write the kickoff prompt for the next task in this workstream branch (not in a worktree).

[START 2025-11-24T10:17:44-05:00] Agent: Codex | Task(s): integration handoff (A+D → F) | Branch: integration/ad | Notes: Merged A_binary_env + D_json_stream_logging; tee/stream/apply docs aligned and ready for version probing.
[END 2025-11-24T10:17:44-05:00] Agent: Codex | Task(s): integration handoff (A+D → F) | Branch: integration/ad | Notes: Handoff to Workstream F.

Kickoff prompt for F:
- Branch from `integration/ad` into `ws/F_versioning_features` (optionally add a worktree) and keep main clean.
- Add capability probing for `codex --version`/`codex features list` with caching keyed by binary path; respect binary/home overrides from Workstream A.
- Guard wrapper flags using detected capabilities and surface upgrade advisories where possible (no network unless host opts in).
- Update docs/examples with the new capability model and run `cargo test -p codex` before merging.
