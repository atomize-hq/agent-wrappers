# Workstream H: Docs and Examples

Objective: Provide clear docs and runnable examples for new capabilities across other workstreams.

Scope
- Update README/EXAMPLES to cover bundled binary + CODEX_HOME override, JSON streaming API, MCP/app-server usage, version detection hooks.
- Add examples demonstrating: exec with images/JSON/schema/output-last-message, CODEX_HOME override, streaming consumer, MCP codex/codex-reply, app-server thread/turn, feature detection.
- Ensure examples compile and are referenced from README.

Constraints
- Coordinate with other workstreams to reflect final APIs.
- Keep examples minimal but functional; avoid external network calls.

Deliverables
- README updates.
- New example files under `crates/codex/examples`.
- Docs snippets per feature.
