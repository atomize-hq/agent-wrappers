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

## Implementation Plan

This slice is not a good candidate for further sub-slicing: it stays within `crates/agent_api`, has three atomic tasks,
and adds only one pure helper + one pure test layer. The work should stay as a single implementation pass, but the
touch surface should be made more concrete than the current optional wording.

#### Preferred touch surface

- `crates/agent_api/src/bounds.rs`
  - Reuse the existing UTF-8-safe truncation machinery already used for event/message/final-text bounds.
  - Promote only the minimal shared pieces needed by MCP bounds to `pub(crate)` helpers/constants.
- `crates/agent_api/src/mcp.rs`
  - Only if S1 has already landed and a thin MCP-specific wrapper or doc comment materially improves discoverability for
    SEAM-3/4 implementers.
- `crates/agent_api/src/lib.rs`
  - No planned logic changes for S3 beyond any import/module plumbing already required by S1.

#### Recommended execution order

1. Canonicalize the shared truncation primitives in `bounds.rs`.
   - Use one shared truncation suffix constant (`…(truncated)`) and one shared UTF-8-safe truncate helper so MCP bounds do
     not drift from existing Universal Agent API bounds behavior.
2. Add a single MCP-focused helper in `bounds.rs`.
   - Preferred signature:
     `pub(crate) fn enforce_mcp_output_bound(bytes: &[u8], saw_more_bytes: bool, bound_bytes: usize) -> (String, bool)`
   - This keeps the byte-capture concern in SEAM-3/4 and the deterministic decode/truncate concern in SEAM-1.
3. Add focused unit tests next to the helper.
   - Most tests should use small synthetic bounds for readability and edge-case precision.
   - Keep one pinned 65,536-byte regression so the spec budget itself is covered directly.
4. Add a short discoverability note for downstream seams.
   - Prefer a doc comment near the helper call surface rather than a second implementation path.

#### Algorithm details that must be pinned in code

- Decode with `String::from_utf8_lossy` (or equivalent) before final truncation so the returned output is always valid
  UTF-8.
- Compute truncation from two independent signals:
  - `saw_more_bytes == true`, or
  - decoded UTF-8 byte length still exceeds `bound_bytes`.
- Preserve the event-envelope fallback rule:
  - if `bound_bytes > suffix.len()`, truncate to `bound_bytes - suffix.len()` bytes and append the suffix;
  - otherwise return `"…"` truncated to `bound_bytes`.
- Never use char-count assertions for correctness; all enforcement must be byte-based.

#### Specific edge case to guard explicitly

Lossy UTF-8 decoding can expand the decoded byte length even when the retained raw byte slice is at or under the bound.
The plan must pin a test where `saw_more_bytes == false` but invalid byte replacement still forces truncation after lossy
decode. This is the easiest place for SEAM-3/4 to drift if the helper is underspecified.

## Atomic Tasks

#### S3.T1 — Implement bounded stdout/stderr enforcement helper (MM-C04)

- **Outcome**: A crate-visible helper that converts captured bytes + “saw more” flag into bounded UTF-8 strings + truncation flags.
- **Inputs/outputs**:
  - Input: `docs/specs/universal-agent-api/mcp-management-spec.md` (“Output capture + truncation algorithm (pinned)”)
  - Output: helper(s) in `crates/agent_api/src/bounds.rs`, optionally wrapped/discoverable from `crates/agent_api/src/mcp.rs`, such as:
    - `enforce_mcp_output_bound(bytes: &[u8], saw_more: bool, bound_bytes: usize) -> (String, bool)`
    - (optional) `enforce_mcp_output(stdout: Captured, stderr: Captured) -> (stdout, stderr, flags)`
- **Implementation notes**:
  - Prefer `crates/agent_api/src/bounds.rs` as the canonical implementation site because the repo already centralizes
    UTF-8-safe truncation behavior there.
  - Promote the smallest possible shared internals (`utf8_truncate_to_bytes`, truncation suffix constant) instead of
    duplicating near-identical logic in `mcp.rs`.
  - If a higher-level `AgentWrapperMcpCommandOutput` constructor is added after S1 lands, it should remain a thin wrapper
    over the `bounds.rs` helper rather than a second implementation.
  - Keeping the helper behind the same `codex` / `claude_code` feature gate as `bounds.rs` is acceptable for this slice,
    because the immediate consumers are the built-in backend mappings in SEAM-3/4 and the verification command already
    runs with those features enabled.
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
  - Include a case where lossy decoding alone pushes the decoded UTF-8 byte length over the bound even though the captured
    raw byte slice is not over-bound.
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
  - Prefer a short doc comment on the helper itself and, if needed, a one-line reference near
    `AgentWrapperMcpCommandOutput` once S1 lands.
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
