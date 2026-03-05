# S3 — Output bounds helper + truncation algorithm tests

- **User/system value**: Pins deterministic, UTF-8-safe stdout/stderr bounds behavior (MM-C04) as shared code so built-in backend mappings (SEAM-3/4) can reuse it without re-implementing truncation semantics.
- **Scope (in/out)**:
  - In:
    - Add SEAM-1-owned helpers that implement the pinned output bounds + truncation algorithm from `docs/specs/universal-agent-api/mcp-management-spec.md`:
      - `stdout` bound: 65,536 bytes
      - `stderr` bound: 65,536 bytes
      - suffix: `…(truncated)`
      - lossy decode if needed; final output always valid UTF-8
      - truncated flag set if the stream exceeded its bound (including “saw more bytes” cases).
    - Add unit tests that pin algorithm edge cases (UTF-8 boundary safety; invalid bytes; “saw more” behavior).
    - Ensure helpers are usable by SEAM-3/4 mapping code (crate-visible surface).
  - Out:
    - The actual bounded streaming capture mechanism for subprocess stdout/stderr (implemented in SEAM-3/4).
    - Cross-backend mapping conformance tests that execute fake binaries (SEAM-5).
- **Acceptance criteria**:
  - A single helper implementation exists in `crates/agent_api` that matches the spec algorithm and is reused by SEAM-3/4.
  - Unit tests cover:
    - under-bound output (no truncation),
    - over-bound output (suffix + truncation),
    - multi-byte Unicode boundary truncation safety,
    - invalid UTF-8 bytes (lossy decode),
    - `saw_more_bytes == true` sets `*_truncated = true` even if the decoded string happens to be under the bound.
  - No semantic drift from the pinned suffix/budget values.
- **Dependencies**:
  - Types from S1: `AgentWrapperMcpCommandOutput`.
- **Verification**:
  - `cargo test -p agent_api --features codex,claude_code`

## Atomic Tasks

#### S3.T1 — Implement bounded stdout/stderr enforcement helper (MM-C04)

- **Outcome**: A crate-visible helper that converts captured bytes + “saw more” flag into bounded UTF-8 strings + truncation flags.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Output capture + truncation algorithm (pinned)”)
  - Output: helper(s) in `crates/agent_api/src/mcp.rs` (or `crates/agent_api/src/bounds.rs`) such as:
    - `enforce_mcp_output_bound(bytes: &[u8], saw_more: bool, bound_bytes: usize) -> (String, bool)`
    - (optional) `enforce_mcp_output(stdout: Captured, stderr: Captured) -> (stdout, stderr, flags)`
- **Implementation notes**:
  - Prefer reusing existing truncation utilities/constants in `crates/agent_api/src/bounds.rs` where feasible to avoid drift (suffix + UTF-8 truncation behavior).
  - If `bounds.rs` remains feature-gated, keep the MCP helper either:
    - in `mcp.rs` (unconditional), or
    - behind the same feature gates as the built-in backends, with unit tests compiled under `--features codex,claude_code`.
- **Acceptance criteria**:
  - Returned strings are always valid UTF-8 and never exceed `bound_bytes` in UTF-8 byte length.
  - Truncation behavior matches suffix and boundary rules.
- **Test notes**: pinned by S3.T2.
- **Risk/rollback notes**: internal helper only; safe.

Checklist:
- Implement: bounded decode + truncate + suffix append logic.
- Test: unit tests (S3.T2).
- Validate: compare behavior to spec algorithm step-by-step.
- Cleanup: keep API small and backend-usable.

#### S3.T2 — Add unit tests pinning the truncation algorithm

- **Outcome**: Deterministic unit tests that cover the pinned algorithm’s edge cases.
- **Inputs/outputs**:
  - Output: tests co-located with the helper (`crates/agent_api/src/mcp.rs` or `crates/agent_api/src/bounds.rs`)
- **Implementation notes**:
  - Include at least one test that:
    - generates output > 65,536 bytes,
    - asserts suffix presence, and
    - asserts UTF-8 validity + max length.
  - Include invalid UTF-8 byte sequences and assert the output contains replacement characters (lossy decode).
  - Include a `saw_more = true` case that forces truncation even when bytes length might otherwise be under-bound.
- **Acceptance criteria**:
  - Tests are stable and deterministic across platforms.
- **Test notes**: pure tests; no subprocess spawning.
- **Risk/rollback notes**: tests-only; safe.

Checklist:
- Implement: test cases for each edge condition.
- Test: `cargo test -p agent_api --features codex,claude_code`.
- Validate: ensure assertions are byte-based (not char count).
- Cleanup: keep tests minimal; avoid large hard-coded blobs (generate instead).

#### S3.T3 — Add minimal integration guidance for downstream mapping seams

- **Outcome**: A short comment/doc note pointing SEAM-3/4 at the helper so they do not reimplement bounds logic.
- **Inputs/outputs**:
  - Output: doc comment in `crates/agent_api/src/mcp.rs` (or a brief note in `docs/specs/universal-agent-api/mcp-management-spec.md` if clarification is needed while the spec is Draft).
- **Implementation notes**:
  - Keep guidance non-normative; the normative algorithm remains in the spec.
- **Acceptance criteria**:
  - A developer working on SEAM-3/4 can discover the helper by search (function name referenced in code comments).
- **Test notes**: n/a.
- **Risk/rollback notes**: docs-only; safe.

Checklist:
- Implement: short comment with helper name + link to spec section.
- Test: compile (doc comment does not break).
- Validate: no duplicate “near-miss” helpers exist.
- Cleanup: keep it short.

## Notes for downstream seams (non-tasking)

- SEAM-3/4 should implement bounded streaming capture (retain `bound_bytes + 1` or `bound_bytes` + `saw_more` flag) and then call the SEAM-1 helper to produce the final bounded strings + flags.
