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
