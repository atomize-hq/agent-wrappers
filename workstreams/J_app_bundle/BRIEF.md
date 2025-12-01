# Workstream J: Bundled Codex Binary & Home Isolation

Objective: Ship an opt-in app-bundled Codex binary flow that never depends on the user's global Codex install, keeps credentials/history isolated per app/project, and makes version pinning/upgrades explicit for host applications.

Scope
- Define a clear bundling contract: host apps resolve a pinned binary from an app-owned bundle root (no PATH/CODEX_BINARY fallback) and pass it to the wrapper.
- Provide a helper to resolve/check an app-scoped bundle path (e.g., `~/.yourapp/codex-bin/<version>/codex`), fail fast when missing, and avoid touching the user's global Codex binary.
- Document how to choose and pass per-project `CODEX_HOME` roots, including where to store/copy `auth.json`/`.credentials.json` when multiple homes are needed.
- Clarify version/update expectations (host app owns download/pinning; wrapper probes capabilities but does not auto-update).

Constraints
- Isolation first: never mutate or depend on the user's global Codex install or HOME when the app-bundle helper is used.
- Keep existing behavior unchanged unless the app opts into the bundled helper (default remains CODEX_BINARY/`codex` on PATH).
- Preserve env safety: the wrapper should still inject `CODEX_BINARY`/`CODEX_HOME` per command without leaking into the parent process.
- Backward compatible docs/examples: make the new flow additive and clearly marked as the recommended app-embedded pattern.

References
- Existing env/binary handling in `crates/codex/src/lib.rs`.
- Auth helper: `AuthSessionHelper` and `login/login_with_api_key/logout` flows.
- Capability probing/caching (binary-path keyed) for optional flag gating.

Deliverables
- Design note and helper API for app-bundled binary resolution (opt-in).
- Documentation/examples showing pinned bundle usage and per-project CODEX_HOME/auth placement.
- Tasks/logs/kickoffs tracked under `workstreams/J_app_bundle/`.
