### S1 — Exec/resume model handoff and argv mapping

- **Decomposition status**: oversized and decomposed for a single-session budget.
- **Why it was split**:
  - touches both `crates/agent_api` and `crates/codex`
  - bundles typed policy plumbing with exec/resume argv verification
- **Directory**: `slice-1-exec-resume-model-handoff/`
- **Archived original**: `archive/slice-1-exec-resume-model-handoff.md`

#### Sub-slices

- `subslice-1-policy-model-handoff.md` (`S1a`): consume SEAM-2's normalized helper output in Codex policy/harness plumbing and keep `None` as the only no-override representation.
- `subslice-2-exec-resume-argv-mapping.md` (`S1b`): thread the typed model into exec/resume request handling and prove exactly-one `--model <trimmed-id>` emission plus ordering through the existing builder path.
