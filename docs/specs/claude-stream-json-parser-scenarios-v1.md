# Claude stream-json Parser Scenarios (v1)

Status: **Normative** (paired with `claude-stream-json-parser-contract.md`)

This document maps fixtures to required outcomes. Fixtures live under:

- `crates/claude_code/tests/fixtures/stream_json/v1/`

## Scenario 1: system init

- Input: `system_init.jsonl`
- Outcome: `ClaudeStreamJsonEvent::SystemInit`

## Scenario 2: system other

- Input: `system_other.jsonl`
- Outcome: `ClaudeStreamJsonEvent::SystemOther`

## Scenario 3: assistant message (text)

- Input: `assistant_message_text.jsonl`
- Outcome: `ClaudeStreamJsonEvent::AssistantMessage`

## Scenario 4: result discriminator (success/error)

- Inputs:
  - `result_success.jsonl` → `ResultSuccess`
  - `result_error.jsonl` → `ResultError`
- These fixtures differ only by `raw["subtype"]` and `raw["is_error"]`.

## Scenario 5: result inconsistency normalize

- Input: `result_inconsistent_is_error.jsonl`
- Outcome: `Normalize` error

## Scenario 6: stream event typed wrapper

- Input: `stream_event_text_delta.jsonl`
- Outcome: `StreamEvent` with `stream.event_type == "content_block_delta"`

## Scenario 7: unknown outer type is not fatal

- Input: `unknown_outer_type.jsonl`
- Outcome: `ClaudeStreamJsonEvent::Unknown`

## Scenario 8: `parse_json` parity with `parse_line`

For every fixture line used in Scenarios 1–7:

- Parse the line into a `serde_json::Value` (after applying the same single-`\r` trimming that
  `parse_line` applies).
- `parse_json(&value)` MUST return the same outcome as `parse_line` on the original line:
  - On success, the same `ClaudeStreamJsonEvent` variant with identical extracted fields.
  - On failure, the same `ClaudeStreamJsonParseError.code` (`TypedParse` or `Normalize`).

This scenario exists to keep the two public entry points in strict lockstep on the same fixture
corpus and normalization semantics.
