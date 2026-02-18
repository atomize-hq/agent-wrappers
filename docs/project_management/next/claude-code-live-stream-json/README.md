# Claude Code live stream-json (Planning Pack)

Source ADR:
- `docs/adr/0010-claude-code-live-stream-json.md`

Key planning artifacts:
- `docs/project_management/next/claude-code-live-stream-json/plan.md`
- `docs/project_management/next/claude-code-live-stream-json/tasks.json`
- `docs/project_management/next/claude-code-live-stream-json/spec_manifest.md`
- `docs/project_management/next/claude-code-live-stream-json/impact_map.md`
- `docs/project_management/next/claude-code-live-stream-json/ci_checkpoint_plan.md`
- `docs/project_management/next/claude-code-live-stream-json/decision_register.md`

Triads:
- `C0`: streaming `--print --output-format stream-json` API in `crates/claude_code` (+ feature smoke workflow/scripts)
- `C1`: `crates/agent_api` Claude backend wiring + `agent_api.events.live`

CI checkpoints:
- Checkpoint plan: `docs/project_management/next/claude-code-live-stream-json/ci_checkpoint_plan.md`
- Checkpoint task: `CP1-ci-checkpoint` (after `C1-integ`)

