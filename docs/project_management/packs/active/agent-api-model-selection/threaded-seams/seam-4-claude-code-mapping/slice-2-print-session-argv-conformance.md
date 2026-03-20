### S2 — Print/session argv conformance and `--fallback-model` exclusion

- Status: decomposed because the original slice spans `crates/agent_api`, `crates/claude_code`, and canonical spec/test surfaces, which is too broad for one Codex session.
- Archived original: `archive/slice-2-print-session-argv-conformance.md`
- Sub-slice directory: `slice-2-print-session-argv-conformance/`

#### Sub-slices

- `subslice-1-agent-api-request-plumbing.md` (`S2a`): thread `model: Option<String>` through Claude backend/harness request construction in `crates/agent_api` and preserve omission semantics without re-parsing or hand-writing argv.
- `subslice-2-claude-print-argv-ordering.md` (`S2b`): pin `ClaudePrintRequest` / root-flags argv behavior in `crates/claude_code` so fresh, resume, and fork flows emit exactly one `--model <trimmed-id>` pair in the required position.
- `subslice-3-fallback-exclusion-and-contract.md` (`S2c`): publish the negative `--fallback-model` contract and align focused docs/tests so the universal key never gains fallback semantics.

#### Audit result

- `slice-1-model-handoff.md`: OK, remains a single bounded plumbing slice.
- `slice-2-print-session-argv-conformance.md`: oversized and decomposed here.
- `slice-3-runtime-rejection-conformance.md`: helper audit also flags this as oversized, but it is outside this requested decomposition pass.
