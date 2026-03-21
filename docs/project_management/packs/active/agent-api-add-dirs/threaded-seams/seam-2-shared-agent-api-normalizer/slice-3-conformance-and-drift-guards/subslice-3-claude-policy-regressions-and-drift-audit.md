### S3c — Pin Claude policy handoff regressions and backend drift guards

- **User/system value**: Claude keeps the same normalized policy contract and cwd fallback semantics as Codex, and the seam closes with an explicit audit that later backend layers are not reintroducing raw add-dir parsing.
- **Scope (in/out)**:
  - In:
    - Add Claude-only direct-policy regressions for normalized `add_dirs` attachment.
    - Pin empty-vector absence semantics and `run_start_cwd` as the final fallback in Claude policy extraction.
    - Finish with the backend drift audit that raw add-dir parsing remains confined to policy extraction surfaces.
  - Out:
    - Codex backend coverage.
    - Claude argv placement, print-request mapping, or capability advertisement.
- **Acceptance criteria**:
  - Claude direct-policy tests assert that absent input yields `policy.add_dirs == Vec::new()`.
  - Relative input resolution proves `request.working_dir -> config.default_working_dir -> run_start_cwd` precedence before helper invocation.
  - The closeout validation checks backend paths for `agent_api.exec.add_dirs.v1` usage and treats any new post-policy parse site as drift.
- **Dependencies**:
  - `S2b`
  - `AD-C02`
  - `AD-C03`
  - `AD-C07`
- **Verification**:
  - `cargo test -p agent_api claude_code`
  - `rg -n "agent_api\\.exec\\.add_dirs\\.v1" crates/agent_api/src/backends`
- **Rollout/safety**:
  - End this sub-slice with the repository drift audit so SEAM-4 and SEAM-5 inherit a clear guardrail against backend-local parsing regressions.

#### S3c.T1 — Add Claude direct-policy regressions and close with a drift audit

- **Outcome**: Claude policy extraction is pinned to the same normalized handoff contract as Codex, and the seam explicitly verifies that later backend modules are still consuming policy state instead of raw payloads.
- **Files**:
  - `crates/agent_api/src/backends/claude_code/tests/`
  - `crates/agent_api/src/backends/`

Checklist:
- Implement:
  - add Claude direct-policy cases for absent key, normalized relative path attachment, and invalid propagation through policy extraction
  - keep assertions focused on `policy.add_dirs` plus `run_start_cwd` as the final cwd fallback
- Test:
  - cover request cwd beating default cwd and default cwd beating run-start cwd for relative inputs
  - assert safe invalid helper errors propagate without raw path leakage
- Validate:
  - run `rg -n "agent_api\\.exec\\.add_dirs\\.v1" crates/agent_api/src/backends` and treat any post-policy parse site as drift to resolve before handoff
  - keep Claude mapping and capability assertions out of this sub-slice
