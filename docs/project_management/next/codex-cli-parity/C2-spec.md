# C2-spec – JSONL + server notification compatibility (fixtures-backed)

Source: `docs/adr/0001-codex-cli-parity-maintenance.md`

## Decisions (no ambiguity)
- Fixture root (exact path): `crates/codex/examples/fixtures/versioned/`
- Required fixture versions for this triad:
  - `0.61.0` (current `cli_manifests/codex/min_supported.txt`)
  - `0.77.0` (ADR 0001 “latest upstream” at time of writing; used as the first drift target)
- Required fixture files per version directory:
  - `crates/codex/examples/fixtures/versioned/<version>/streaming.jsonl`
  - `crates/codex/examples/fixtures/versioned/<version>/resume.jsonl`
  - `crates/codex/examples/fixtures/versioned/<version>/malformed.jsonl` (must include at least 1 invalid JSON line and 1 valid line after it)
- New test file (exact path): `crates/codex/tests/jsonl_compat.rs`
- Normalization/compat documentation file (exact path): `crates/codex/JSONL_COMPAT.md`

## Task Breakdown (no ambiguity)
- `C2-code` (non-test changes):
  - Implement drift-tolerant parsing/normalization and unknown-field capture for the existing JSONL stream surface.
  - Write `crates/codex/JSONL_COMPAT.md` documenting normalization + error surfacing.
- `C2-test` (tests only):
  - Add fixtures under `crates/codex/examples/fixtures/versioned/` (per-version directories + required files).
  - Add tests in `crates/codex/tests/jsonl_compat.rs` that cover: known-good parsing, unknown-field retention, malformed-line non-fatal behavior.
- `C2-integ`:
  - Merge `C2-code` + `C2-test`, reconcile to this spec, and run `cargo fmt`, `cargo clippy ...`, `cargo test -p codex`, and `make preflight`.

## Scope
- Treat Codex CLI `--json` event streams as **versioned and drift-prone**, especially for older binaries.
- Implement compatibility behavior per ADR 0001:
  - Prefer typed event parsing, but tolerate schema drift via normalization and unknown-field capture.
  - Do not fail the entire stream on the first parse/normalize error; surface errors to the caller while continuing to read remaining events when possible.
  - Maintain a fixtures-based sample corpus for JSONL and server notifications, refreshed when CLI behavior changes.
- Cover both:
  - CLI JSONL events (`codex ... --json`)
  - Server notification schemas (MCP/app-server) as applicable to this repo’s wrapper surface

### Scope narrowing (to keep this triad execution-ready)
- For “notifications”, this triad covers only the JSONL events consumed by this crate’s existing typed stream surface (`ThreadEvent` parsing used by `CodexClient::stream_exec` / `CodexClient::stream_resume`).
- Do not add new networked integration tests; use fixtures and unit/integration tests only.

## Acceptance Criteria
- The fixtures corpus exists at the exact paths specified in “Decisions (no ambiguity)”.
- Tests exist that:
  - parse known-good events across fixtures,
  - assert unknown fields are captured/retained (not dropped silently),
  - assert a malformed/unrecognized event does not terminate the entire stream (it is reported and parsing continues).
- Normalization rules are documented in `crates/codex/JSONL_COMPAT.md` with at least:
  - what is normalized,
  - when normalization is applied (version gates/heuristics),
  - what errors are surfaced to callers.

## Out of Scope
- Expanding to non-Codex CLIs (explicit non-goal in ADR 0001).
- Live network calls or credentialed probes as part of default tests (live probes must remain opt-in per ADR).
- Rewriting public API surfaces unrelated to JSONL/notification compatibility.
