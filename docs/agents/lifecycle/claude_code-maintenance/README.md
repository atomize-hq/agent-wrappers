<!-- generated-by: xtask agent-maintenance renderer; source-of-truth: governance/maintenance-request.toml -->

# claude_code maintenance

This packet tracks automated upstream-release maintenance for `claude_code`.

## Request

- request artifact: `docs/agents/lifecycle/claude_code-maintenance/governance/maintenance-request.toml`
- trigger kind: `upstream_release_detected`
- basis ref: `cli_manifests/claude_code/latest_validated.txt`
- opened from: `.github/workflows/agent-maintenance-open-pr.yml`
- recorded at: `2026-07-24T12:27:32Z`
- request commit: `9400ee8ee5da2f7813a33129472e29b822b6e5bf`

## Trigger context

- detected_by: `.github/workflows/agent-maintenance-release-watch.yml`
- current_validated: `2.1.29`
- target_version: `2.1.206`
- latest_stable: `2.1.206`
- version_policy: `upstream_stable_pointer`
- source_kind: `npm_dist_tag`
- source_ref: `@anthropic-ai/claude-code#stable`
- dispatch_kind: `packet_pr`
- dispatch_workflow: `agent-maintenance-open-pr.yml`
- branch_name: `automation/claude_code-maintenance-2.1.206`

## Support-surface audit

- required: `true`
- pre-run debt count: `2`
- expected post-run debt count: `2`
- discovered upstream surface rows: `0`
- preexisting unsupported rows: `2`
- required uplifts this run:
- none
- deferred preexisting gaps:
- `claude install` `install` via `requires_new_architectural_seam` (TODOS.md#close-claude-code-install-maintenance-gap)
- `claude install` `--force` via `requires_new_architectural_seam` (TODOS.md#close-claude-code-install-maintenance-gap)


## Canonical execution contract

Use `docs/agents/lifecycle/claude_code-maintenance/HANDOFF.md` as the exact contributor execution contract for this lane. The PR body summary under `docs/agents/lifecycle/claude_code-maintenance/governance/pr-summary.md` is derivative only.
