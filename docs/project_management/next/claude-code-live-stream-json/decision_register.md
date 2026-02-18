# Decision Register — Claude Code live stream-json

Status: Draft  
Date (UTC): 2026-02-17  
Feature directory: `docs/project_management/next/claude-code-live-stream-json/`

This register records the non-trivial architectural decisions required to make ADR-0010 execution-ready.
Each decision is exactly two options (A/B) with explicit tradeoffs and one selection.

## DR-0001 — Where to implement Claude stream-json live streaming

**A) Implement streaming in `crates/agent_api` by spawning `claude` directly**
- Pros: avoids expanding `crates/claude_code` public surface; keeps “universal API” logic local.
- Cons: duplicates CLI spawning/env/timeout logic that already exists in `crates/claude_code`; risks drift between the wrapper crate and universal backend; harder to reuse streaming in non-`agent_api` consumers.

**B) Implement streaming in `crates/claude_code` as a first-class API (Selected)**
- Pros: single source of truth for spawning + env + timeout + parsing; reusable for non-`agent_api` consumers; keeps `agent_api` backend thin.
- Cons: adds a new public streaming handle/API surface that must be supported and tested.

**Selected:** B

## DR-0002 — Per-line parse error behavior while streaming

**A) Fail-fast: any JSONL parse error aborts the run**
- Pros: simplest semantics; easier debugging when upstream output is malformed.
- Cons: fragile; a single malformed line can discard subsequent valid events; worse operator ergonomics for long runs.

**B) Degrade: emit a redacted error outcome and continue parsing subsequent lines (Selected)**
- Pros: robust; preserves best-effort progress; matches “best-effort” posture for heterogeneous CLIs.
- Cons: requires careful redaction/bounds rules; may hide upstream bugs if not surfaced clearly.

**Selected:** B

## DR-0003 — Streaming API item type

**A) Stream only successful typed events (`Stream<Item = ClaudeStreamJsonEvent>`) and route parse errors elsewhere**
- Pros: simplifies event consumers; fewer `Result` types.
- Cons: requires an additional error channel or sideband; complicates ordering guarantees; risks lost parse errors.

**B) Stream `Result<ClaudeStreamJsonEvent, ClaudeStreamJsonParseError>` in-order (Selected)**
- Pros: preserves ordering between events and parse errors; one stream for consumers; matches “parse errors are part of the stream” intuition.
- Cons: consumers must handle `Result`; forces a stable parse-error type surface.

**Selected:** B

## DR-0004 — Streaming completion payload

**A) Completion yields only `ExitStatus` (Selected)**
- Pros: avoids buffering stdout/stderr; preserves the v1 “no raw backend line capture” posture; simplest contract.
- Cons: makes debugging harder without separate capture; consumers needing stdout/stderr must capture out-of-band.

**B) Completion yields full `CommandOutput` (status + stdout/stderr)**
- Pros: richer debugging; mirrors existing `claude_code` `print(...)` behavior.
- Cons: implies retaining raw backend output (stdout/stderr) in memory; higher secret-leak risk; larger compatibility surface.

**Selected:** A

## DR-0005 — CI checkpoint workflow strategy for this feature

**A) Add a dedicated feature-local smoke workflow for this feature (Selected)**
- Pros: deterministic cross-platform evidence on GitHub-hosted runners; no coupling to unrelated feature workflows; easiest to run while feature is in-flight on its own branch.
- Cons: workflow duplication (another smoke workflow) unless later consolidated.

**B) Reuse/extend an existing workflow (e.g., `universal-agent-api-smoke.yml` or `ci.yml`)**
- Pros: fewer workflows; avoids duplicating “fmt/clippy/test” logic.
- Cons: coupling across features; `workflow_dispatch` limitations when the workflow is not on the default branch; increases churn in unrelated workflows.

**Selected:** A

## DR-0006 — Claude live-stream capability ids exposed via `agent_api`

**A) Use only core `agent_api.events.live` (Selected)**
- Pros: minimal capability surface; aligns with the universal “live streaming” signal; avoids capability proliferation.
- Cons: loses a backend-specific “how” signal unless consumers also inspect existing backend capability ids.

**B) Add a backend-specific live capability id (e.g., `backend.claude_code.print_stream_json.live`)**
- Pros: explicit backend-mechanism signal; can be useful for debugging/telemetry and capability snapshots.
- Cons: adds another stable id to support; increases docs and test surface.

**Selected:** A

## DR-0007 — Completion semantics for non-zero exit status in the streaming API

**A) Completion resolves `Ok(ExitStatus)` regardless of success/failure (Selected)**
- Pros: aligns with existing `agent_api` completion contract (status is the primary result); keeps error channel for spawn/I/O/timeout/cancellation failures; minimizes behavior drift vs Codex backend.
- Cons: consumers must interpret non-zero statuses themselves.

**B) Treat non-zero exit as `Err(ClaudeCodeError::BackendFailure { ... })`**
- Pros: “failure” is surfaced as an error by default; may be simpler for some consumers.
- Cons: blurs “process finished” vs “transport failed”; complicates parity with Codex; requires designing and stabilizing a new error variant.

**Selected:** A

## DR-0008 — Streaming API dependency + structuring strategy

**A) Use `tokio::sync::mpsc` + `futures_core::Stream` with a small custom stream wrapper (Selected)**
- Pros: matches patterns already used in `crates/agent_api` (custom `Stream` impl, no extra stream crate); minimal new deps (`futures-core` only); keeps tokio-centric spawning consistent with existing wrapper code.
- Cons: `crates/claude_code` gains a dependency on `futures-core` if it does not already have one.

**B) Add a stream utility crate (e.g., `tokio-stream`) and expose `ReceiverStream`-style wrappers**
- Pros: less custom code for stream wrappers; common ecosystem tool.
- Cons: adds an additional dependency and pattern surface; diverges from the existing `agent_api` approach.

**Selected:** A

## DR-0009 — Backpressure behavior when streaming JSONL events

**A) Apply backpressure (block on bounded channel send) (Selected)**
- Pros: deterministic “no drops” behavior; bounded memory; keeps ordering; aligns with safety posture and DR-0012 completion gating (if you want completion, drain the stream or drop it).
- Cons: a slow/paused consumer can cause the reader task to stop draining stdout, which can in turn cause the child process to block on a full pipe (expected).

**B) Drop events under load (best-effort streaming)**
- Pros: keeps stdout draining even if consumer is slow; avoids child pipe backpressure.
- Cons: violates “no implicit loss” expectations for event streams; makes debugging and completion semantics surprising; requires explicit drop metrics/policies.

**Selected:** A

## DR-0010 — Stderr handling for live streaming (`claude --print --output-format stream-json`)

**A) Drain stderr but do not retain it (Selected)**
- Pros: avoids deadlocks; preserves v1 safety posture (no raw backend line retention); aligns with Codex adapter behavior (stderr discarded by default).
- Cons: less post-mortem debugging info unless the operator opts into mirroring to console.

**B) Capture stderr bytes and return in completion**
- Pros: richer debugging surface.
- Cons: implies retaining raw backend output; higher secret-leak risk; increases memory use and stable contract surface; conflicts with DR-0004.

**Selected:** A
