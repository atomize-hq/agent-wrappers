# Feature Template (Triads)

Copy this directory to create a new feature/sprint under `docs/project_management/next/<feature>/`, then replace placeholders like `<feature>` and `<feature-prefix>`.

Expected contents:
- `plan.md` — guardrails + triad overview + start/end checklists.
- `tasks.json` — triad tasks (code/test/integration) with explicit checklists, dependencies, and kickoff prompt paths.
- `session_log.md` — START/END log entries only (edited only on the orchestration branch).
- `C*-spec.md` — one spec per triad, defining scope/acceptance/out-of-scope.
- `kickoff_prompts/` — one prompt per triad role.

Canonical process: `docs/project_management/task-triads-feature-setup-standard.md`.
