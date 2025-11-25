# Codex Docs/Examples Handoff (H6)

## Highlights
- Documented the JSON stream schema across README/rustdoc/examples (thread/turn lifecycle, item types including reasoning/command_execution/file_change/mcp_tool_call/web_search/todo_list, failure events), with updated `--sample` payloads carrying IDs/status and a `turn.failed` example.
- Cleaned EXAMPLES index to match shipped samples (removed stale ingestion row), clarified streaming/logging rows, and added MCP/app-server notification notes (thread/turn IDs in samples).
- Feature detection example now gates artifact flags and MCP/app-server flows alongside streaming/log tee, with richer sample capability sets.

## Risks / Gaps
- No typed streaming API in the library yet; only examples consume JSONL and log tee to files. Sample payloads may drift from real Codex output without an integration fixture.
- MCP/app-server examples rely on mocked notifications when the binary is absent; no automated coverage of codex/codex-reply approval/cancel flows or conversation lifecycle.
- Capability probing remains demo-only (no caching or builder integration), and feature names may diverge from future `codex features list` output; update advisories still focus on streaming/log tee.
- Artifact handling (`--output-last-message/--output-schema`) and CODEX_HOME layout are documented but not validated against a live binary; apply/resume/apply/diff flows remain undocumented here.

## Verification
- `cargo test -p codex --doc` (pass)
- `cargo test -p codex --examples` (pass)

## H7 backlog sweep
- Added `resume_apply` example plus README/lib/EXAMPLES coverage for resume/diff/apply flows, apply stdout/stderr/exit capture, and log-tee advisories.
- Synced feature detection docs/examples with per-binary caching and gating for resume/apply/artifact flags alongside streaming/log-tee and server endpoints.
- Remaining risks: streaming/apply payloads are still sample-only (no live binary validation), feature names from `codex features list` may drift, and a typed streaming API inside the crate remains unimplemented.

## H8 post-review
- Streaming samples now include thread/turn IDs and `item.updated` coverage, and docs call out `thread.resumed` events plus `apply.result` shapes so resume/apply flows mirror the current CLI surface.
- Residual risks: streaming/apply payloads remain mocked (no live binary verification), feature names may change between releases, and the wrapper still buffers JSONL rather than exposing a typed stream API.
- Upgrade guidance: gate streaming/log-tee/resume/apply/artifact flags behind feature detection, refresh per-binary capability caches when the Codex binary path/version changes, and keep `--sample` fallbacks handy when the binary is unavailable.

## H9 integration fixture
- Added shared streaming/resume/apply fixtures under `crates/codex/examples/fixtures/*` to drive `--sample` flows and docs, plus a fixture sanity test to catch invalid JSON drift.
- Docs now point at the fixtures and call out per-binary cache refresh guidance; examples reuse the fixtures for streaming/log tee/resume+apply samples.
- Remaining gaps: fixtures are still mocked (not captured from a live binary), and the crate still buffers JSONL instead of exposing a typed streaming API.
