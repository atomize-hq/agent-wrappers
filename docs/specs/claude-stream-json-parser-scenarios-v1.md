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

