### S2b — Codex fork rejection boundary + invalid-input precedence

- **User/system value**: proves the Codex fork path keeps add-dir handling inside the pinned safe
  pre-request rejection boundary, while malformed inputs still fail earlier as `InvalidRequest`.
- **Scope (in/out)**:
  - In:
    - Selector `"last"` accepted-input rejection coverage before any `thread/list`,
      `thread/fork`, or `turn/start` request.
    - Selector `"id"` accepted-input rejection coverage before any `thread/fork` or `turn/start`
      request.
    - Malformed / missing / non-directory add-dir payload precedence tests on the same fork
      surfaces.
    - Explicit zero-request assertions against the fake Codex app-server binary.
    - Safe-message alignment with `AD-C03` / `AD-C08`.
  - Out:
    - Codex exec/resume argv ordering coverage (`S2a`).
    - Claude selector-branch placement coverage (`S2c`).
    - Post-handle runtime rejection parity (`S3`).
- **Acceptance criteria**:
  - Accepted add-dir inputs on fork selector `"last"` reject before any `thread/list`,
    `thread/fork`, or `turn/start` request is sent.
  - Accepted add-dir inputs on fork selector `"id"` reject before any `thread/fork` or
    `turn/start` request is sent.
  - Malformed / missing / non-directory add-dir payloads fail as exact safe `InvalidRequest`
    messages before the Codex-specific backend rejection path.
  - Tests prove the zero-request boundary explicitly rather than inferring it from a generic error.
- **Dependencies**:
  - SEAM-3 Codex fork boundary (`AD-C08`)
  - `AD-C04` session-flow parity from the threading registry
  - `AD-C03` safe error posture for exact redacted messages
- **Verification**:
  - `cargo test -p agent_api --all-features codex`
- **Rollout/safety**:
  - Test-only sub-slice. Keep accepted-input rejection and invalid-input precedence in separate
    assertions so future regressions show which boundary drifted.

#### S2b.T1 — Add Codex fork branch coverage for rejection boundary and invalid-input precedence

- **Outcome**: Codex fork coverage pins the accepted-input exception, invalid-input precedence, and
  zero-request app-server boundary for both selector branches.
- **Files**:
  - `crates/agent_api/src/backends/codex/tests/**`
  - `crates/agent_api/src/bin/fake_codex_app_server_jsonrpc_agent_api.rs`
  - Evidence-only input: `crates/agent_api/src/backends/codex/fork.rs`

Checklist:
- Implement:
  - Add accepted-input rejection tests for selectors `"last"` and `"id"` separately.
  - Add malformed-payload precedence tests for the same fork surfaces.
  - Make the app-server fixture fail loudly if `thread/list`, `thread/fork`, or `turn/start` is
    reached on a rejection path that should stop earlier.
  - Keep safe-message assertions aligned with the exact `AD-C03` / `AD-C08` wording.
- Test:
  - Run `cargo test -p agent_api --all-features codex`.
- Validate:
  - Confirm zero app-server requests are sent on both accepted-input rejection paths.
  - Confirm malformed payloads fail before the Codex-specific backend rejection branch.
  - Confirm no raw path content leaks into the pinned rejection messages.
