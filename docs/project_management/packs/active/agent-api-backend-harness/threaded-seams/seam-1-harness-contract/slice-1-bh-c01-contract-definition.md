# S1 — Pin `BH-C01` adapter contract as code

- **User/system value**: Unblocks downstream seams by freezing the internal “identity + supported extension keys + spawn + typed mapping + completion extraction” contract shape.
- **Scope (in/out)**:
  - In:
    - Define `BH-C01 backend harness adapter interface` as internal Rust API.
    - Define minimal supporting types/aliases needed to express the interface.
    - Define explicit error-mapping boundaries (redaction points) as part of the interface.
  - Out:
    - Implementing canonical normalization/validation policy (`BH-C02`, SEAM-2).
    - Implementing streaming pump + drain-on-drop semantics (`BH-C04`, SEAM-3).
    - Implementing DR-0012 completion gating integration (`BH-C05`, SEAM-4).
    - Migrating real backends to the harness (SEAM-5).
- **Acceptance criteria**:
  - `BH-C01` exists and is `pub(crate)` in `crates/agent_api/src/backend_harness.rs`.
  - The interface covers:
    - backend identity/kind,
    - supported extension keys surface + backend-specific validation hook surface,
    - spawn returning `(typed stream, completion future)` (or equivalent),
    - typed-event → `AgentWrapperEvent` mapping hook,
    - explicit backend error → `AgentWrapperError` mapping hook(s) at spawn/stream/completion boundaries.
  - Clean build under `--features codex`, `--features claude_code`, and combined.
- **Dependencies**: none.
- **Verification**:
  - `cargo check -p agent_api --features codex`
  - `cargo check -p agent_api --features claude_code`
  - `cargo check -p agent_api --features codex,claude_code`

## Canonical internal Rust API (BH-C01) (normative for this pack)

The canonical internal harness module is:

- `crates/agent_api/src/backend_harness.rs` (internal-only)

The canonical interface name is:

- `BackendHarnessAdapter` (a `pub(crate)` trait)

The harness entrypoint that backends call is:

- `backend_harness::run_harnessed_backend(...) -> Result<AgentWrapperRunHandle, AgentWrapperError>`

### Type aliases (exact)

```rust
use std::{collections::BTreeMap, future::Future, pin::Pin, time::Duration};

use futures_core::Stream;

use crate::{
    AgentWrapperCompletion, AgentWrapperError, AgentWrapperEvent, AgentWrapperKind,
    AgentWrapperRunHandle, AgentWrapperRunRequest,
};

pub(crate) type DynBackendEventStream<E, BE> =
    Pin<Box<dyn Stream<Item = Result<E, BE>> + Send + 'static>>;

pub(crate) type DynBackendCompletionFuture<C, BE> =
    Pin<Box<dyn Future<Output = Result<C, BE>> + Send + 'static>>;
```

### Phase enum (exact; required for error mapping)

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BackendHarnessErrorPhase {
    Spawn,
    Stream,
    Completion,
}
```

### Spawn output (exact)

```rust
pub(crate) struct BackendSpawn<E, C, BE> {
    pub events: DynBackendEventStream<E, BE>,
    pub completion: DynBackendCompletionFuture<C, BE>,
}
```

### Backend defaults (normalized inputs)

The harness normalization step consumes only the following backend defaults in v1:

```rust
#[derive(Clone, Debug, Default)]
pub(crate) struct BackendDefaults {
    pub env: BTreeMap<String, String>,
    pub default_timeout: Option<Duration>,
}
```

Other backend config defaults (e.g., `working_dir`, binary paths, etc.) are **not** part of SEAM-2
normalization in this pack and remain backend-owned until an explicit contract adds them.

### Normalized request (exact)

```rust
pub(crate) struct NormalizedRequest<P> {
    /// Stable identity for error reporting and event stamping.
    pub agent_kind: AgentWrapperKind,

    /// Preserved from `AgentWrapperRunRequest` (must be non-empty after trimming).
    pub prompt: String,

    /// Preserved from `AgentWrapperRunRequest` (no harness defaulting in v1).
    pub working_dir: Option<std::path::PathBuf>,

    /// Derived per BH-C03. `Some(Duration::ZERO)` is an explicit “no timeout” request.
    pub effective_timeout: Option<Duration>,

    /// Derived per BH-C03: `defaults.env` overridden by `request.env`.
    pub env: BTreeMap<String, String>,

    /// Backend-owned extracted policy derived from `request.extensions` after the allowlist check.
    pub policy: P,
}
```

### Adapter trait (exact signatures + bounds)

```rust
pub(crate) trait BackendHarnessAdapter: Send + Sync + 'static {
    /// MUST return a stable, lower_snake_case id (see `AgentWrapperKind` rules).
    fn kind(&self) -> AgentWrapperKind;

    /// Supported extension keys for this backend (exact string match; case-sensitive).
    ///
    /// This list MUST include both:
    /// - core keys under `agent_api.*` that the backend supports, and
    /// - backend keys under `backend.<agent_kind>.*` owned by the backend.
    fn supported_extension_keys(&self) -> &'static [&'static str];

    /// Backend-owned policy extracted from known extension keys only.
    ///
    /// This hook MUST NOT implement “unknown key” rejection (that is BH-C02, harness-owned).
    type Policy: Send + 'static;

    fn validate_and_extract_policy(
        &self,
        request: &AgentWrapperRunRequest,
    ) -> Result<Self::Policy, AgentWrapperError>;

    /// Typed backend event and completion types emitted by the wrapper runtime.
    type BackendEvent: Send + 'static;
    type BackendCompletion: Send + 'static;

    /// Backend error type used at spawn/stream/completion boundaries.
    type BackendError: Send + Sync + 'static;

    /// Spawns the backend run using only the normalized request.
    ///
    /// The returned stream MUST be drained to completion by the harness pump (BH-C04).
    fn spawn(
        &self,
        req: NormalizedRequest<Self::Policy>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output =
                        Result<BackendSpawn<Self::BackendEvent, Self::BackendCompletion, Self::BackendError>, Self::BackendError>,
                > + Send
                + 'static,
        >,
    >;

    /// Maps one typed backend event into 0..N universal events.
    ///
    /// Mapping is **infallible** by contract: backends MUST convert parse errors into
    /// `BackendError` at the stream boundary, not here.
    fn map_event(&self, event: Self::BackendEvent) -> Vec<AgentWrapperEvent>;

    /// Maps a typed backend completion value to the universal completion payload.
    fn map_completion(
        &self,
        completion: Self::BackendCompletion,
    ) -> Result<AgentWrapperCompletion, AgentWrapperError>;

    /// Produces a safe/redacted message for a backend error at a given phase.
    ///
    /// This message MUST NOT contain raw backend stdout/stderr lines or raw JSONL lines.
    /// It MAY include bounded metadata such as `line_bytes=<n>` or a stable error kind tag.
    fn redact_error(&self, phase: BackendHarnessErrorPhase, err: &Self::BackendError) -> String;
}
```

### Harness entrypoint (exact signature)

```rust
pub(crate) fn run_harnessed_backend<A: BackendHarnessAdapter>(
    adapter: std::sync::Arc<A>,
    defaults: BackendDefaults,
    request: AgentWrapperRunRequest,
) -> Result<AgentWrapperRunHandle, AgentWrapperError>;
```

Notes:
- `agent_kind` for `AgentWrapperError::{UnsupportedCapability, ...}` MUST be rendered as
  `adapter.kind().as_str().to_string()` (stable, lowercase; no `Debug` formatting).
- All universal events emitted by the harness MUST stamp `agent_kind = adapter.kind()`.

## Error propagation + redaction + bounds (BH-C01 contract requirements)

This section pins the behavior required by downstream seams so the harness can be implemented and
tested deterministically.

### Redaction / safety requirements (normative)

`BackendHarnessAdapter::redact_error(...)` MUST:

- return a stable, operator-safe message suitable for:
  - `AgentWrapperEventKind::Error.message`, and
  - `AgentWrapperError::Backend { message }`.
- MUST NOT contain:
  - raw backend stdout/stderr lines, or
  - raw JSONL/event lines emitted by the backend CLI.
- MAY contain bounded metadata such as:
  - `line_bytes=<n>` (length only),
  - a stable `error_kind` tag (`"spawn" | "timeout" | "parse" | ...`),
  - `status=<ExitStatus>` formatted via `{:?}` when it does not include raw output.

### Bounds enforcement contract (normative)

The harness MUST enforce universal bounds exactly once (harness-owned), using the existing
implementation in `crates/agent_api/src/bounds.rs` (constants + algorithms):

- For every universal event produced by `map_event(...)` (and for any harness-synthesized error
  events), the harness MUST apply:
  - `crate::bounds::enforce_event_bounds(event)` and forward **each** resulting bounded event.
- For every universal completion produced by `map_completion(...)`, the harness MUST apply:
  - `crate::bounds::enforce_completion_bounds(completion)` before publishing it as the run outcome.

Backends MUST NOT perform a second bounds pass after migrating to the harness (SEAM-5), to avoid
accidental semantic drift (double-splitting or double-truncation).

### Error propagation model (v1; pinned)

The harness has three error boundaries for `BackendError`:

1) **Spawn** (`BackendHarnessErrorPhase::Spawn`)
   - If `spawn(...)` returns `Err(err)`:
     - The harness MUST synthesize **one** universal `Error` event with:
       - `kind = AgentWrapperEventKind::Error`
       - `message = Some(adapter.redact_error(Spawn, &err))`
       - `data = None`
     - The harness MUST publish the run outcome as:
       - `Err(AgentWrapperError::Backend { message })` (same message as the event).
     - Finality:
       - There is no backend stream to drain; the pump MUST drop the event sender immediately
         after attempting the error-event send (stream is final at that point).

2) **Stream item** (`BackendHarnessErrorPhase::Stream`)
   - If the typed stream yields `Err(err)`:
     - The harness MUST synthesize a universal `Error` event (same schema as above) and treat it
       as a **non-fatal** stream event.
     - The harness MUST continue draining the backend stream to completion (BH-C04), regardless
       of whether the receiver is alive.

3) **Completion** (`BackendHarnessErrorPhase::Completion`)
   - The completion sender task MUST await the backend completion future to a `Result<C, BackendError>`.
   - If it yields `Err(err)`:
     - The completion sender MUST publish the run outcome as:
       - `Err(AgentWrapperError::Backend { message })` where `message =
         adapter.redact_error(Completion, &err)`.
     - The completion sender MUST also attempt to synthesize and send a universal `Error` event
       (best-effort) using the same message. The pump continues draining independently.

#### “Winner” rule when multiple errors occur

- Completion outcome is authoritative for `AgentWrapperRunHandle.completion`:
  - Stream errors NEVER override the completion outcome.
  - If spawn fails, the completion outcome is that spawn failure.

#### Interaction with DR-0012 completion gating

- The completion sender publishes completion outcome as soon as it is known (independent of draining).
- Observability of that completion outcome remains gated by `run_handle_gate`:
  - completion MUST NOT resolve until (a) the event stream is final or (b) the consumer drops the
    events stream.

### Pinned tests required by this contract

These tests are required to keep the safety contract executable and prevent accidental leakage:

- Bounds enforcement algorithms are pinned by existing unit tests in:
  - `crates/agent_api/src/bounds.rs` (message truncation, text splitting, data oversize replacement).
- Raw backend output leakage prevention MUST be pinned by backend-local tests:
  - Codex: add a test in `crates/agent_api/src/backends/codex/tests.rs` that constructs an
    `ExecStreamError::Parse { line: "SECRET...", ... }` and asserts that the resulting
    `redact_exec_stream_error(...)` output does **not** contain the raw line content (only
    `line_bytes=<n>` metadata is permitted).
  - Claude Code: add an analogous test in `crates/agent_api/src/backends/claude_code/tests.rs` for
    `redact_parse_error(...)` (no raw JSONL line capture in messages).
- “Winner” behavior (completion is authoritative) MUST be pinned by a harness-level test:
  - Add `completion_error_wins_over_stream_errors` in `crates/agent_api/src/backend_harness.rs`:
    - typed stream yields at least one `Err(BackendError)` and then terminates,
    - completion future resolves to `Err(BackendError)`,
    - assert: the run’s completion outcome is an `Err(AgentWrapperError::Backend { .. })` derived
      from the completion error (not the earlier stream error), and the event stream still reaches finality.

## Atomic Tasks

#### S1.T1 — Define `BH-C01` interface + supporting types

- **Outcome**: A minimal adapter contract (`BackendHarnessAdapter`) and spawn/result types that can represent “typed stream + typed completion + mapping”.
- **Inputs/outputs**:
  - Output: `crates/agent_api/src/backend_harness.rs` (new)
  - Output (wiring): `crates/agent_api/src/lib.rs` (`mod backend_harness;` + `pub(crate)` re-exports if needed)
- **Implementation notes**:
  - Keep everything `pub(crate)` and co-located for auditability.
  - Prefer referencing existing public types (no new public API):
    - `AgentWrapperKind`, `AgentWrapperCapabilities`, `AgentWrapperRunRequest`,
    - `AgentWrapperEvent`, `AgentWrapperCompletion`, `AgentWrapperError`.
  - Represent spawn output explicitly as a `(Stream<Item = Result<TypedEvent, BackendErr>>, Future<Output = Result<TypedCompletion, BackendErr>>)`-like shape with `Send` bounds.
- **Acceptance criteria**:
  - The contract is small enough to review quickly (avoid generic “framework” abstractions).
  - No lifetime gymnastics needed for a backend adapter to implement it.
- **Test notes**: exercised by Slice S2 toy adapter smoke tests.
- **Risk/rollback notes**: internal-only; can be iterated without breaking public API.

Checklist:
- Implement: `BH-C01` trait/struct + minimal spawn types/aliases.
- Test: compile the `agent_api` crate with all relevant feature flags.
- Validate: `make clippy` (warnings are errors) on the workspace.
- Cleanup: ensure the module is clearly internal (no `pub` exports).

#### S1.T2 — Define supported extension keys + backend-specific validation hook surfaces (no enforcement yet)

- **Outcome**: The harness can ask a backend “what extension keys do you support?” and can call a backend-provided validator for backend-specific extension payload semantics.
- **Inputs/outputs**:
  - Output: additions in `crates/agent_api/src/backend_harness.rs`.
- **Implementation notes**:
  - Include a supported-extension-keys accessor as part of the adapter contract.
  - Include a backend-specific validator hook surface that can reject malformed backend-specific payloads.
  - Do **not** implement unknown-key rejection logic here; enforcement/policy is `BH-C02` (SEAM-2).
- **Acceptance criteria**:
  - Downstream seams can implement fail-closed validation without each backend re-implementing allowlists.
  - The boundary between “backend-specific validation” (this seam) and “unknown-key rejection” (SEAM-2) is explicit in docs/comments.
- **Test notes**: toy adapter provides a small allowlist + validator hook that is invoked pre-spawn.
- **Risk/rollback notes**: keep hook minimal; avoid overfitting to current Codex/Claude extension sets.

Checklist:
- Implement: `supported_extension_keys()` + `validate_and_extract_backend_policy()` hook.
- Test: call ordering is possible (validate-before-spawn) in the harness lifecycle.
- Validate: no policy logic creeps in (unknown-key rejection stays for SEAM-2).
- Cleanup: document the ownership split (`BH-C02` vs backend-specific validation).

#### S1.T3 — Define canonical error-mapping points (redaction boundary)

- **Outcome**: The contract has explicit hooks for mapping backend-specific errors into `AgentWrapperError` (or bounded/redacted messages) at spawn/stream/completion boundaries.
- **Inputs/outputs**:
  - Output: additions in `crates/agent_api/src/backend_harness.rs`.
- **Implementation notes**:
  - Prefer a single mapping surface with phase context (spawn/stream/completion) rather than scattered ad-hoc conversions.
  - Document intent: prevent leaking raw backend lines or internal error detail into universal envelope semantics.
- **Acceptance criteria**:
  - The harness has a canonical way to map backend failures; downstream seams do not introduce divergent error formatting.
  - Smoke tests can invoke the mapper in at least one boundary.
- **Test notes**: toy adapter returns a sentinel backend error; mapping produces stable `AgentWrapperError::Backend { message: ... }`.
- **Risk/rollback notes**: internal-only; can be refined without public API impact.

Checklist:
- Implement: error mapping hook(s) + optional phase enum.
- Test: a simulated spawn failure maps deterministically.
- Validate: no `Debug` dumps of backend errors are surfaced by default.
- Cleanup: keep the mapping API small and explicit.
