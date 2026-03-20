# S1 — Capability, policy extraction, and root-flags mapping

- **Status**: Decomposed into sub-slices sized for one Codex session each.
- **Audit result**: Oversized. This slice combines capability publication, policy-state wiring,
  fresh-run argv mapping, backend-local regression coverage, and contract-doc pinning across
  `mod.rs`, `backend.rs`, `harness.rs`, `tests/capabilities.rs`,
  `tests/backend_contract.rs`, and `docs/specs/claude-code-session-mapping-contract.md`.
- **Sub-slice directory**:
  `slice-1-capability-policy-and-root-flags/`
- **Archived original**:
  `archive/slice-1-capability-policy-and-root-flags.md`

#### Sub-slices

- `slice-1-capability-policy-and-root-flags/subslice-1-capability-surface.md`
  - `S1a` covers original `S1.T1`: publish `agent_api.exec.add_dirs.v1` on Claude capability and
    allowlist surfaces, plus capability-sync assertions.
- `slice-1-capability-policy-and-root-flags/subslice-2-policy-extraction.md`
  - `S1b` covers original `S1.T2`: normalize and store Claude add-dir policy state using the
    shared helper and the effective working directory.
- `slice-1-capability-policy-and-root-flags/subslice-3-root-flags-mapping-and-docs.md`
  - `S1c` covers original `S1.T3`: emit the fresh-run variadic `--add-dir <DIR...>` group in
    pinned order and update the Claude mapping contract/doc-backed ordering assertions.

#### Execution Order

1. `S1a` to align capability and supported-key surfaces.
2. `S1b` to make the advertised key real in Claude policy extraction.
3. `S1c` to pin argv ordering and the backend-owned contract text once the policy field exists.
